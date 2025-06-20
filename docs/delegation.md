# Delegation

You can run matrixon on a separate domain than the actual server name (what shows up in user ids, aliases, etc.).
For example you can have your users have IDs such as `@foo:example.org` and have aliases like `#bar:example.org`,
while actually having matrixon hosted on the `matrix.example.org` domain. This is called delegation.

## Automatic (recommended)

matrixon has support for hosting delegation files by itself, and by default uses it to serve federation traffic on port 443.

With this method, you need to direct requests to `/.well-known/matrix/*` to matrixon in your reverse proxy.

This is only recommended if matrixon is on the same physical server as the server which serves your server name (e.g. example.org)
as servers don't always seem to cache the response, leading to slower response times otherwise, but it should also work if you
are connected to the server running matrixon using something like a VPN.

To configure it, use the following options in the `global.well_known` table:
| Field | Type | Description | Default |
| --- | --- | --- | --- |
| `client` | `String` | The URL that clients should use to connect to matrixon | `https://<server_name>` |
| `server` | `String` | The hostname and port servers should use to connect to matrixon | `<server_name>:443` |

### Example

```toml
[global.well_known]
client = "https://matrix.example.org"
server = "matrix.example.org:443"
```

## Manual

Alternatively you can serve static JSON files to inform clients and servers how to connect to matrixon.

### Servers

For servers to discover how to access your domain, serve a response in the following format for `/.well-known/matrix/server`:

```json
{
  "m.server": "matrix.example.org:443"
}
```
Where `matrix.example.org` is the domain and `443` is the port matrixon is accessible at.

### Clients

For clients to discover how to access your domain, serve a response in the following format for `/.well-known/matrix/client`:
```json
{
  "m.NextServer": {
    "base_url": "https://matrix.example.org"
  }
}
```
Where `matrix.example.org` is the URL matrixon is accessible at.

To ensure that all clients can access this endpoint, it is recommended you set the following headers for this endpoint:
```
Access-Control-Allow-Origin: *
Access-Control-Allow-Methods: GET, POST, PUT, DELETE, OPTIONS
Access-Control-Allow-Headers: X-Requested-With, Content-Type, Authorization
```
