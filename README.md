# syncme

Sync selected files and directories between machines over SSH with `rsync`

`syncme` keeps one JSON config per machine, tracks absolute paths, and mirrors them
between configured machines. A machine can use an always-on **hub** that stores the
files and fans them out, or sync directly with its **peers**.

> [!Warning]
> This software is still early stage. Expect breaking changes. Read [Safety](#safety)

## Requirements

- `rsync`
- SSH access to the configured hub or peers
- Rust toolchain, if building from source

## Topology

There are three ways to configure a machine:

- **Hub**: `role: "hub"`. The hub has no `hub` address of its own and pushes to
  every name in `peers`. It keeps a mirror of everything peers push to it, then
  fans that mirror back out; peers that are offline receive it on a later run.
  This is the recommended mode for an always-on server.
- **Peer with hub**: `role: "peer"` and `hub` set. The peer pushes only to the
  hub and pulls only from the hub. Its `peers` list is topology metadata; the
  hub's `peers` list is authoritative for fan-out.
- **Peer-to-peer**: `role: "peer"` and `hub: null`. The peer pushes to every
  name in `peers` and pulls from one explicitly named peer.

The hub must have the union of all tracked paths in its `paths` list; otherwise
a path tracked only on one peer is never fanned back out. Use cron or a
supervisor to run `syncme push` periodically on the hub.

With a hub configured, `push` requires the hub to be reachable; `peers` are not
a fallback. If the hub is offline, retry `syncme push` once it returns.

## Config

Config is stored in `$XDG_CONFIG_HOME/syncme-conf.json` or
`$HOME/.config/syncme-conf.json`.

### Hub config (for `hub-host`)

```json
{
  "role": "hub",
  "peers": ["laptop", "desktop"],
  "paths": [
    "/home/user/.bashrc",
    "/home/user/.config/"
  ]
}
```

### Peer with hub

```json
{
  "role": "peer",
  "hub": "hub-host",
  "peers": [],
  "paths": [
    "/home/user/.bashrc",
    "/home/user/.config/"
  ]
}
```

### Peer-to-peer config

```json
{
  "role": "peer",
  "hub": null,
  "peers": ["laptop", "desktop"],
  "paths": [
    "/home/user/.bashrc",
    "/home/user/.config/"
  ]
}
```

Create a config with:

```sh
syncme config init          # peer without hub (P2P)
syncme config init hub-host    # peer with hub
syncme config init --hub    # hub
```

Manage peers with `syncme peer ...`.

# Safety

`push` and `pull` use `rsync --delete` for directories: the destination directory
is a **complete mirror**, and files not present in the source are removed.

- There are no exclude patterns yet.
- Peer-to-peer mode has no automatic conflict resolution: the last peer to push
  wins. Do not list a machine as its own peer.

Until data protection is implemented, try the tool with disposable data or at
your own risk.
