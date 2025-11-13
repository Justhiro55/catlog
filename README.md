# catlog

Monitor logs and display cat images when HTTP errors are detected.

## Installation

```bash
cargo install --path .
```

## Usage

### Pipe mode
```bash
your-command | catlog
```

### File watch mode
```bash
catlog -f /var/log/app.log
```

### Command execution mode
```bash
catlog -e "tail -f /var/log/app.log"
```

## Options

- `-f, --follow <FILE>`: Follow a file (like tail -f)
- `-e, --exec <COMMAND>`: Execute a command and monitor its output
- `--size <N>`: Image size in characters (default: 60)
- `--no-image`: Don't display images (text only)
- `--all`: Show cats for all status codes
- `--status <CODES>`: Comma-separated list of specific status codes

## Examples

```bash
# Monitor nginx logs
tail -f /var/log/nginx/access.log | catlog

# Watch application logs
catlog -f app.log

# Only show specific status codes
echo "Error 404" | catlog --status 404,503
```
