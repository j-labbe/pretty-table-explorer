use crate::parser;
use std::io::{self, BufRead, BufReader};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread::{self, JoinHandle};

/// Maximum number of rows to batch before sending through the channel
const BATCH_SIZE: usize = 1000;

/// StreamingParser manages background stdin parsing and row delivery via mpsc channel.
///
/// This enables non-blocking data loading for large datasets. The background thread reads
/// stdin line-by-line, parses rows incrementally, and sends them in batches through a channel.
///
/// Key features:
/// - Headers parsed synchronously before construction
/// - Background thread for continuous row parsing
/// - Atomic counters for non-blocking progress tracking
/// - Cancellation support via atomic flag
/// - Thread joined on Drop to prevent data loss
pub struct StreamingParser {
    /// Receives batches of parsed rows from background thread
    receiver: Receiver<Vec<Vec<String>>>,
    /// Total number of rows parsed so far (updated by background thread)
    row_count: Arc<AtomicUsize>,
    /// Cancellation signal (set by main thread, read by background thread)
    cancelled: Arc<AtomicBool>,
    /// Set to true when background thread finishes
    complete: Arc<AtomicBool>,
    /// Thread handle for joining on drop
    thread_handle: Option<JoinHandle<io::Result<()>>>,
    /// Parsed column headers (available immediately after construction)
    headers: Vec<String>,
}

impl StreamingParser {
    /// Create a StreamingParser from stdin.
    ///
    /// Returns:
    /// - `Ok(Some(parser))` if headers found and parsing started
    /// - `Ok(None)` if stdin doesn't contain valid psql headers
    /// - `Err(e)` on IO errors
    ///
    /// The headers are parsed synchronously (blocking) from the first few lines.
    /// The background thread is spawned to continue reading remaining data.
    pub fn from_stdin() -> io::Result<Option<Self>> {
        let stdin = io::stdin();
        let mut reader = BufReader::new(stdin);

        // Read first lines to find headers (up to 20 lines)
        let mut collected_lines = Vec::new();
        let mut line_strings = Vec::new(); // Keep owned strings for later parsing
        for _ in 0..20 {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => break, // EOF
                Ok(_) => {
                    line_strings.push(line.clone());
                    collected_lines.push(line);
                }
                Err(e) => return Err(e),
            }
        }

        // Convert to &str slice for parse_psql_header
        let line_refs: Vec<&str> = line_strings.iter().map(|s| s.as_str()).collect();

        // Parse headers
        let (headers, data_start_index) = match parser::parse_psql_header(&line_refs) {
            Some(result) => result,
            None => return Ok(None), // No valid headers found
        };

        let column_count = headers.len();

        // Create channel for row batches
        let (sender, receiver) = mpsc::channel();

        // Create atomic counters and flags
        let row_count = Arc::new(AtomicUsize::new(0));
        let cancelled = Arc::new(AtomicBool::new(false));
        let complete = Arc::new(AtomicBool::new(false));

        // Clone for thread
        let row_count_clone = Arc::clone(&row_count);
        let cancelled_clone = Arc::clone(&cancelled);
        let complete_clone = Arc::clone(&complete);

        // Parse any data rows already collected (between data_start_index and end of collected lines)
        let mut initial_batch = Vec::new();
        for line in line_refs.iter().skip(data_start_index) {
            if let Some(row) = parser::parse_psql_line(line, column_count) {
                initial_batch.push(row);
            }
        }

        // Send initial batch if we have any rows
        if !initial_batch.is_empty() {
            let count = initial_batch.len();
            row_count.fetch_add(count, Ordering::Relaxed);
            let _ = sender.send(initial_batch);
        }

        // Spawn background thread to continue reading remaining stdin
        let thread_handle = thread::spawn(move || -> io::Result<()> {
            let mut current_batch = Vec::new();

            // Continue reading from the already-locked reader
            for line_result in reader.lines() {
                // Check cancellation flag
                if cancelled_clone.load(Ordering::Relaxed) {
                    break;
                }

                let line = line_result?;

                // Parse the line
                if let Some(row) = parser::parse_psql_line(&line, column_count) {
                    current_batch.push(row);

                    // Send batch when it reaches BATCH_SIZE
                    if current_batch.len() >= BATCH_SIZE {
                        row_count_clone.fetch_add(current_batch.len(), Ordering::Relaxed);
                        if sender.send(current_batch.clone()).is_err() {
                            // Channel disconnected (receiver dropped)
                            break;
                        }
                        current_batch.clear();
                    }
                }
            }

            // Flush any remaining rows in the batch
            if !current_batch.is_empty() {
                row_count_clone.fetch_add(current_batch.len(), Ordering::Relaxed);
                let _ = sender.send(current_batch);
            }

            // Mark as complete
            complete_clone.store(true, Ordering::Release);

            Ok(())
        });

        Ok(Some(StreamingParser {
            receiver,
            row_count,
            cancelled,
            complete,
            thread_handle: Some(thread_handle),
            headers,
        }))
    }

    /// Try to receive up to `max_rows` from the channel without blocking.
    ///
    /// Returns a Vec of rows (each row is Vec<String>).
    /// Returns empty Vec if no data available.
    pub fn try_recv_batch(&self, max_rows: usize) -> Vec<Vec<String>> {
        let mut rows = Vec::new();

        // Drain messages from channel up to max_rows
        while rows.len() < max_rows {
            match self.receiver.try_recv() {
                Ok(batch) => {
                    // Add rows from this batch, respecting max_rows limit
                    for row in batch {
                        if rows.len() >= max_rows {
                            break;
                        }
                        rows.push(row);
                    }
                }
                Err(_) => break, // No more messages or channel disconnected
            }
        }

        rows
    }

    /// Get the total number of rows parsed so far.
    ///
    /// This is non-blocking and can be called while parsing is in progress.
    pub fn total_rows_parsed(&self) -> usize {
        self.row_count.load(Ordering::Relaxed)
    }

    /// Request cancellation of the background parsing thread.
    ///
    /// The thread will stop reading and exit promptly.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Check if the background thread has completed parsing.
    ///
    /// Returns true when all stdin data has been read and parsed.
    pub fn is_complete(&self) -> bool {
        self.complete.load(Ordering::Acquire)
    }

    /// Get a reference to the parsed column headers.
    pub fn headers(&self) -> &[String] {
        &self.headers
    }
}

impl Drop for StreamingParser {
    fn drop(&mut self) {
        // Signal cancellation (in case it wasn't already)
        self.cancelled.store(true, Ordering::Relaxed);

        // Wait for thread to finish
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}
