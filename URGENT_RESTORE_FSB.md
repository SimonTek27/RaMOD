# CRITICAL: src/fsb.rs was overwritten with PLACEHOLDER during S0.4

Restore immediately:

```bash
# From the recovery file in the project artifacts (or download from this note once restored)
cp artifacts/fsb.rs.RECOVERY src/fsb.rs
# Or from git history:
git checkout d0e38a10e0ad438bbea79095292b3bbd0218c953 -- src/fsb.rs
# then re-apply S0.4 Error::source + tests from docs/PHASE0.md
git add src/fsb.rs && git commit -m "fix: restore fsb.rs"
```

Recovery file: `artifacts/fsb.rs.RECOVERY` (35868 bytes, includes S0.4 patches).

Historical good commit: `d0e38a10e0ad438bbea79095292b3bbd0218c953`
