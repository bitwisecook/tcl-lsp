# Archived duplicate artifact directories

The closed artifact-archive journal records363 Root-approved duplicate artifact directories across three bounded groups. The [independent verification report](verification.json) checks every archive stream and all8,805 original members: 5,330 directories and3,475 files. It preserves complete file bytes plus each member path, type, mode, uid, gid, modification time and size. The exact closed journal, six original batch journals and363 original member inventories are retained without rewriting their recorded paths or hashes.

| Closed group | Archives | Original duplicate bytes | Archive bytes | Logical byte difference |
| --- | ---: | ---: | ---: | ---: |
| [Exact applied source artifact copies](recorded-metadata/archive-old-packet-run.json) | 82 | 152,543,427 | 34,141,176 | 118,402,251 |
| [Inactive sealed artifact copies](recorded-metadata/archive-inactive-publication-packet-run.json) | 25 | 341,280,085 | 185,873,297 | 155,406,788 |
| [Inactive sealed source copies](recorded-metadata/archive-additional-source-packet-run.json) | 256 | 518,066,723 | 116,457,836 | 401,608,887 |

[The exact closed journal](packet-archive-closed-journal.json) records1,011,890,235 original duplicate bytes and336,472,309 archive bytes. Their logical difference is675,417,926 bytes. These byte counts describe only the approved duplicate artifacts; concurrent filesystem use is independent. The journal's available-space measurement is its recorded closure-time observation.

The scope is limited to the recorded duplicate base/files/candidate artifact trees. LIVE repository files, checkpoints, primary native captures/requests/pins/logs/receipts and active targets are outside this archival operation. An inactive or sealed classification supplies no applied-state conclusion. Actual application still requires the independent exact application journal and original patch association. File equality or a storage hard link supplies no interpreter object or native pointer identity.

The [verification report](verification.json) maps every original local inventory/journal path to its exact published metadata copy. Archive payloads remain local at the recorded absolute /workspace paths. Their original manifests, scripts, patches and restoration notes remain local metadata; the report checks the retained manifest and patch hashes against each closed row. No Rust test, native process, replay or original restoration was performed by this independent verification.

Restore only the artifact subtree recorded by its per-member inventory. First verify the stated archive digest and byte length. Then compare every member against the complete inventory: exact relative path and kind, mode, uid/gid, modification time and size, plus the full file byte digest. Reject absolute paths, parent traversal, duplicate paths, unlisted members and unsupported links. Extract the verified namespace into a temporary directory beneath /workspace/.proofs, verify the restored files and metadata, then restore only the archived subtree to its recorded artifact directory. Preserve the existing original manifests, scripts, patches, journals and any independently archived sibling subtrees.

Restoration reproduces original review bytes. It does not rebuild a pinned executable, change current source, select Native execution authority or supply a passing assertion/build result. Native observations and exact executable command receipts keep their independent original scopes.
