# syncme

Sync selected files and directories between machines over SSH with `rsync`

`syncme` keeps one JSON config per machine, tracks absolute paths, and mirros them
between a destignated **main** machine and its peers. Directories are mirrored
(extras are deleted).

> [!Warning]
> This software is still early stage. Expect breaking changes. Read [Safety](#safety)

## Requirements

- Linux (at the current stage, it relies on `/etc/hostname`)
- `rsync`
- Rust toolchain, if building from source

## Config

Config is stored in `$XDG_CACHE_HOME/syncme-conf.json` or `$HOME/.config/syncme-conf.json`

### Example config file

```json
{
  "this_machine": "laptop",
  "main_machine": "server",
  "connected_machines": [
    "server",
    "user@host",
    "desktop"
  ],
  "paths": [
    "/home/user/.bashrc",
    "/home/user/.config/"
  ]
}
```

# Safety

`push` and `pull` use `rsync --delete` for directories: the destination directory
is a **complete mirror**, and files not present in the source are removed.

- There are no exclude patterns yet.

Until these are addressed, try the tool with disposable data or at your own risk.
