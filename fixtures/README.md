# Test fixtures

Disposable packages used by the manual test matrix. They install one script to
`/usr/bin` and nothing else. **Do not ship them.**

```
./fixtures/build.sh
```

Produces in `fixtures/build/`:

| File | Exercises |
| --- | --- |
| `hello-debdrop_1.0.0_all.deb` | install, remove, reinstall |
| `hello-debdrop_1.1.0_all.deb` | upgrade |
| `hello-debdrop_0.9.0_all.deb` | downgrade (`--allow-downgrades`) |
| `debdrop-noscripts_1.0.0_all.deb` | no maintainer-scripts warning |
| `debdrop-baddep_1.0.0_all.deb` | unresolved dependency (offline failure) |
| `debdrop-badpostinst_1.0.0_all.deb` | postinst exits 1, install failure |
| `debdrop-debconf_1.0.0_all.deb` | debconf templates note |

Manual sanity check before using them in the app:

```
dpkg-deb -f fixtures/build/hello-debdrop_1.0.0_all.deb
dpkg-deb --info fixtures/build/hello-debdrop_1.0.0_all.deb
```

Clean up an accidental install with `sudo apt-get remove hello-debdrop`.
