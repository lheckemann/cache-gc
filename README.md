# Nix binary cache garbage collector

Hacked-together approach to collecting garbage from local flat-file
binary caches.

Build the package, and run `bin/cache-gc` with the path to the binary
cache. It will compute the closures of all the paths in the cache, and
compute which paths can be deleted (are not referenced by any paths
newer than `--days`, 90 by default):

```
result/bin/cache-gc --days 30 /var/lib/hydra/cache
```

For automated use, `--delete` can be passed, which does not require
confirmation before actually deleting files. Use with care!
