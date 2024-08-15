# Train Backend API endpoints

## `/ws` Websocket Endpoint

- For basic viewing and interaction.
- Client receives `<server_packet>` and sends `<client_packet>` (described in `grammar.bnf`).

## `/ws-ctrl` Websocket Endpoint

- For editing stuff.
- Client receives nothing and sends `<ctrl_packet>` (described in `grammar.bnf`).
- Client should also connect `/ws` for information.

## `/force-derail` HTTP Endpoint

- `GET /force-derail` cause the backend process to reset and load the default nodes, tracks, and trains.
  - Which is currently nothing.
- `GET /force-derail/{track}` cause the backend process to reset and load a specified set of nodes and tracks.
  - `{track}` is a valid track name, possibly chosen from the list return by the `/available-tracks` HTTP Endpoint.
  - if `{track}` is invalid (determined by the backend process), the default stuff are used.

## `/available-tracks` HTTP Endpoint

- `GET /available-tracks` return a list of available train tracks to choose fromm
- returned json is a array of strings containing only ascii alphanumerics, "_", or "-".
