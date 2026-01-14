# pte - Pretty Table Explorer

An interactive terminal-based table viewer for PostgreSQL. Provides smooth scrolling and clean rendering of query results with support for both piped psql output and direct database connections.

## Installation

### One-line install

```sh
curl -fsSL https://raw.githubusercontent.com/yourusername/pretty-table-explorer/master/install.sh | sh
```

### Custom install directory

```sh
INSTALL_DIR=~/.local/bin curl -fsSL https://raw.githubusercontent.com/yourusername/pretty-table-explorer/master/install.sh | sh
```

### Manual download

Download the binary for your platform from [GitHub Releases](https://github.com/yourusername/pretty-table-explorer/releases/latest):

- Linux x86_64: `pte-linux-x86_64`
- Linux ARM64: `pte-linux-aarch64`
- macOS Intel: `pte-macos-x86_64`
- macOS Apple Silicon: `pte-macos-aarch64`

Then make it executable and move to your PATH:

```sh
chmod +x pte-*
sudo mv pte-* /usr/local/bin/pte
```

## Usage

### Pipe psql output

```sh
psql -h localhost -d mydb -c "SELECT * FROM users" | pte
```

### Direct connection

```sh
pte --connect "host=localhost dbname=mydb user=postgres"
```

## Navigation

- `h/j/k/l` or arrow keys: Navigate
- `g`: Go to top
- `G`: Go to bottom
- `/`: Search/filter rows
- `q`: Quit

## License

MIT
