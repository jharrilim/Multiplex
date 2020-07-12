# Multiplex

Multiplex is a key-value store with the ability to publish and subscribe
to keys. Uses REST.

## Running the Application

Runs on http://localhost:8080

```sh
cargo run
```

## Commands

### GET

#### [GET] /store/{key}

Get's a value that was set for this key.

##### Example Request

```sh
curl http://localhost:8080/store/cow
```

##### Example Response

```sh
moo
```

### SET

#### [POST] /store/{key}

Set's a value for the given key using a raw string in the request body.

##### Example Request

```sh
curl -d "moo" http://localhost:8080/store/cow
```