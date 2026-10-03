# Sample library edge cases

The edge-case libraries are listed in `../steamdex.json` and can be run with
`steamdex parse "<library name>"`.

Some fixtures need filesystem features that Git does not preserve as ordinary
files:

- `sample-library-rom-symlink` contains a file symlink and a directory symlink.
- `sample-library-rom-broken-symlink` contains a symlink to a missing file.
- `sample-library-rom-permission-denied/restricted` should have no permissions.

If symlinks or the directory permissions were not preserved on checkout, from
the repository root run:

```sh
ln -s direct-target.rom test-data/sample-library-rom-symlink/file-link.rom
ln -s linked-folder test-data/sample-library-rom-symlink/directory-link
ln -s does-not-exist.rom test-data/sample-library-rom-broken-symlink/missing-target.rom
chmod 000 test-data/sample-library-rom-permission-denied/restricted
```

Permission-denied behavior depends on the OS and the user running Steamdex; an
administrator or root user may still be able to read the restricted directory.
