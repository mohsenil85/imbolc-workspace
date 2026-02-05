# Old Sibling Repo Recovery

The previous standalone `imbolc-core` repo was archived before this repo was created. The backup contains extra commits not present in this repo.

## Backup Location

```
~/imbolc-core-old-backup.tar.gz
```

## Extra Commits

These commits existed in the old repo but are not in this one. They were additional extraction refactors that moved UI helper logic into core:

| Hash | Description |
|------|-------------|
| `43ab994` | Extract instrument structure navigation |
| `fda2f24` | Extract selection region normalization into grid module |
| `fb2e44d` | Extract grid/zoom utilities |
| `37ce465` | Extract parameter adjustment algorithms |

These moved instrument navigation, grid/zoom calculations, and param adjustment algorithms from the TUI binary into ilex.

## How to Cherry-Pick

1. Extract the backup:
   ```bash
   mkdir /tmp/imbolc-core-old
   tar xzf ~/imbolc-core-old-backup.tar.gz -C /tmp/imbolc-core-old
   ```

2. Inspect the commits:
   ```bash
   cd /tmp/imbolc-core-old/imbolc-core
   git log --oneline
   ```

3. Create patches:
   ```bash
   git format-patch 43ab994~1..43ab994 -o /tmp/patches
   # or for a range:
   git format-patch 37ce465..43ab994 -o /tmp/patches
   ```

4. Apply to this repo:
   ```bash
   cd /path/to/ilex
   git am /tmp/patches/*.patch
   ```
