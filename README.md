# Multiplex

## Start

Runs on http://localhost:8080

```sh
cargo run
```

## Routes

### [GET] /get/{key}

Get's a value that was set for this key.

### [SET] /set/{key}

Set's a value for the given key using a raw string in the request body.