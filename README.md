# Backup upload server
This is a simple server with an endpoint to upload backup files.
It is secured with a token.

Note: the base of the source code was AI-generated

## Run
Create `.env`:
```
cp .env.example .env
```

Then edit the `.env` file.

### Development
```
cargo run
```

### Production
TODO

## Endpoints
```
/upload
/health
```

### Examples
```
curl http://localhost:8080/health
```

```
curl \
    -X POST \
    -H "Authorization: your-secret-token" \
    -F "file=@filename.ext" \
    http://localhost:8080/upload
```
