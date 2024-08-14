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
  - Which is currently a single node and nothing else.
