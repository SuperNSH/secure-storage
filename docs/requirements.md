# NSH secure storage: requirements from upstream triage

Each requirement comes from the upstream issues and pull requests in `docs/upstream-triage.md`. Numbers are `flutter_secure_storage` issues (#) and pull requests (PR #). Platforms are `core`, `apple`, `android`, `windows`, `linux`, `web`, `typescript`. Each requirement has a named test, which must be able to fail. The design is in `docs/brief-triage.md`.

Differences from upstream, decided here and recorded in this file: `reset_on_error` defaults to false (upstream PR #1306 says its option defaults to true); no legacy migration exists, since NSH has no old data.

## R1. Web: concurrent first writes share one key

- Upstream issues: #1289
- Upstream PRs: PR #1297
- Platforms: web
- Behavior: On a fresh store, concurrent writes create one AES key. Before storing a new key, check again and use the stored one. Every written value reads back.
- Test: `web_concurrent_first_writes_share_one_key`

## R2. A failed lookup never replaces or deletes a key or data

- Upstream issues: #13, #32, #43, #46, #54, #58, #105, #159, #169, #173, #175, #179, #191, #201, #239, #258, #304, #312, #313, #347, #352, #357, #358, #363, #392, #441, #501, #595, #622, #632, #653, #677, #700, #789, #797, #875, #959, #1064, #1068, #1115, #1138, #1165, #1298
- Upstream PRs: PR #23, PR #47, PR #66, PR #664, PR #861, PR #946, PR #975, PR #1148, PR #1306
- Platforms: core, android, apple, windows, linux
- Behavior: A transient Keystore, Keychain, Credential Manager or keyring failure returns a retryable `store_unavailable` error with the key name. Only an explicit permanent signal may delete, and only with reset_on_error = true: Android KeyPermanentlyInvalidatedException or a failed AEAD tag.
- Test: `transient_lookup_failure_keeps_key_and_data; android_null_keystore_read_with_stored_data_keeps_key`

## R3. reset_on_error defaults to false and runs at most once

- Upstream issues: #279, #941, #1027, #1282
- Upstream PRs: PR #280, PR #587, PR #865, PR #1028, PR #1284
- Platforms: core, android
- Behavior: reset_on_error defaults to false, unlike upstream (PR #1306's text says upstream defaults to true). With true, one reset per call, no recursion. If a fresh key also fails, return `reset_failed`.
- Test: `reset_on_error_defaults_to_false; reset_on_error_runs_at_most_once`

## R4. A locked keystore is a retryable error, not a key mismatch

- Upstream issues: #279, #941, #1027, #1282
- Upstream PRs: PR #280, PR #587, PR #865, PR #1028, PR #1284
- Platforms: android
- Behavior: UserNotAuthenticatedException and a locked keystore return a retryable `locked` error. They never enter the key-mismatch or reset path, because a new key cannot fix them.
- Test: `user_not_authenticated_is_locked_not_mismatch`

## R5. Per-operation biometrics survive every key change

- Upstream issues: #474, #576, #615, #1285, #1303
- Upstream PRs: PR #878, PR #1143, PR #1167, PR #1264, PR #1271, PR #1294
- Platforms: android
- Behavior: With requireBiometricsPerOperation, every read, write and delete authenticates. This holds after first initialization and after a migration. No cipher is cached across calls.
- Test: `per_operation_biometric_prompts_every_call`

## R6. An invalidated biometric key has a code and an opt-in recovery

- Upstream issues: #474, #576, #615, #1285, #1303
- Upstream PRs: PR #1143, PR #1167, PR #1264, PR #1271, PR #1294
- Platforms: android
- Behavior: An invalidated key returns `key_invalidated` with the key name. Recovery needs an explicit opt-in (reset_on_error, or a reset_storage call) that deletes the Keystore key and the data together. It works in per-operation mode too.
- Test: `invalidated_biometric_key_reports_then_recovers_on_opt_in`

## R7. Linux writes are read back, and rejected stores are errors

- Upstream issues: none
- Upstream PRs: PR #407, PR #1182, PR #1290
- Platforms: linux
- Behavior: After each write, read the item back. On mismatch, retry once with the whole map. A rejected store or delete returns an error and never success. Keys not touched by a dropped replace are kept.
- Test: `linux_write_verifies_readback_and_reports_rejection; linux_write_keeps_other_keys_when_replace_is_dropped`

## R8. Linux keyring failures are coded, and the schema follows relabelling

- Upstream issues: #309, #353, #438, #464, #530, #545, #618, #643, #702, #747, #778, #951, #1181, #1203, #1230
- Upstream PRs: PR #617, PR #621, PR #1132, PR #1163, PR #1177, PR #1204, PR #1207, PR #1223, PR #1249
- Platforms: linux
- Behavior: A locked keyring, missing default collection, cold keyring, AppArmor or sandbox denial returns `keyring_locked`, `keyring_unavailable` or `access_denied`. A failure never reads as null. The libsecret schema attribute matches the store after a relabel.
- Test: `linux_locked_keyring_is_coded_error; linux_schema_attribute_follows_relabel`

## R9. Handles that share a store serialize initialization

- Upstream issues: #982
- Upstream PRs: PR #165, PR #170, PR #334, PR #1261, PR #1299
- Platforms: core, android
- Behavior: One lock per store identity, held through initialization. A second engine or handle waits and then uses the first one's key material. Initialized handles do not take the lock.
- Test: `two_handles_same_store_serialize_init`

## R10. Backend load and native failures become coded errors

- Upstream issues: #73, #81, #85, #100, #144, #168, #189, #190, #195, #206, #207, #209, #303, #380, #484, #554, #555, #583, #713, #794, #802, #832, #883, #901, #910, #937, #953, #966, #1075, #1084, #1094, #1145, #1208, #1276, #1283
- Upstream PRs: PR #145, PR #1080, PR #1209
- Platforms: core, android, apple, windows, linux, web
- Behavior: A missing platform class (NoClassDefFoundError, NoSuchFieldError), a linkage failure or a native crash at the boundary returns `backend_unavailable`. The process does not abort.
- Test: `backend_load_failure_is_coded_error`

## R11. Store identity isolates data; deleteAll touches only its own keys

- Upstream issues: #22, #399, #449, #563, #607, #619, #912, #914, #1023, #1026, #1109, #1126, #1144
- Upstream PRs: PR #184, PR #292, PR #731, PR #979, PR #1031, PR #1051, PR #1231, PR #1265, PR #1266
- Platforms: core, android, apple, windows, linux, web
- Behavior: The store name and key prefix isolate data on every backend. deleteAll removes only keys under its own prefix. Web deleteAll removes only the store's keys, not all of localStorage. Options under one name are honoured.
- Test: `delete_all_touches_only_own_prefix; store_names_isolate_data`

## R12. Apple write is an atomic update, never delete then add

- Upstream issues: #188, #496, #573, #581, #711, #785, #835, #1011, #1229
- Upstream PRs: PR #751, PR #1012
- Platforms: apple
- Behavior: Write runs SecItemUpdate. Only on errSecItemNotFound does it run SecItemAdd. No duplicate items are created, and a failed add never removes the old value.
- Test: `apple_write_overwrites_without_delete`

## R13. Apple query identity: items are found across accessibility, hot restart and updates

- Upstream issues: #28, #70, #107, #155, #264, #284, #324, #346, #350, #369, #391, #406, #415, #434, #524, #532, #578, #586, #659, #686, #692, #720, #752, #760, #762, #837, #849, #949, #1013, #1030, #1137, #1158, #1164
- Upstream PRs: PR #56, PR #465, PR #602, PR #628, PR #712, PR #718, PR #719, PR #736, PR #854, PR #868, PR #1136, PR #1183, PR #1269
- Platforms: apple
- Behavior: read, write, delete and containsKey build one query from the service and access group. Accessibility is applied on write only and is not a search filter. An item written at any accessibility level is found after a hot restart and after an app update.
- Test: `apple_read_finds_item_written_with_other_accessibility`

## R14. containsKey agrees with read on every platform

- Upstream issues: #39, #127, #266, #302, #338, #359, #456
- Upstream PRs: PR #33, PR #139, PR #271, PR #273, PR #274, PR #295, PR #297, PR #485, PR #737
- Platforms: core (all backends)
- Behavior: containsKey is true exactly when read returns a value. It is false after delete and for an absent key on every backend.
- Test: `contains_key_matches_read_on_every_platform`

## R15. An absent key is None, not an error

- Upstream issues: #84, #277, #285, #287
- Upstream PRs: PR #288, PR #326, PR #570
- Platforms: core, android, apple, windows, linux, web
- Behavior: read of an absent key returns None. delete of an absent key succeeds. Deleting from a folder that does not exist succeeds.
- Test: `read_and_delete_absent_key_are_not_errors`

## R16. An empty string is a value, distinct from absent

- Upstream issues: none
- Upstream PRs: PR #95
- Platforms: core (all backends)
- Behavior: An empty string is stored and read back as an empty value, not as absent. Writing null is not a write; delete is the only removal.
- Test: `empty_value_round_trips_and_is_not_absent`

## R17. Reserved and non-ASCII key names are encoded safely

- Upstream issues: #539
- Upstream PRs: none
- Platforms: windows, linux, android, web, apple
- Behavior: Keys with colon, slash, backslash, '..', Unicode, emoji and more than 255 characters map to backend-safe names. They never crash and never leave the store directory.
- Test: `reserved_key_characters_round_trip`

## R18. Values of 1 MiB round-trip, and the size limit is documented

- Upstream issues: #55, #171, #1050
- Upstream PRs: PR #194
- Platforms: core, all platforms
- Behavior: A value of 1 MiB round-trips on every platform. The documented limit is the one the tests prove.
- Test: `large_value_round_trips_on_every_platform`

## R19. Windows: each app has its own Credential Manager target

- Upstream issues: #294, #306, #521, #605, #943, #980
- Upstream PRs: PR #325
- Platforms: windows
- Behavior: The Credential Manager target name and the values file include the app identity. Two apps never read each other's values.
- Test: `windows_target_name_includes_app_identity`

## R20. Windows: values file is written atomically

- Upstream issues: #275, #307, #634, #757, #813, #922, #1046
- Upstream PRs: none
- Platforms: windows
- Behavior: Write to a temporary file, then rename over the old one. A locked file is retried, then reported as `file_locked`. A failed write never truncates the file.
- Test: `windows_values_write_is_atomic`

## R21. Apple accessibility: safe default, one-to-one mapping, documented

- Upstream issues: #50, #51, #54, #68, #82, #88, #102, #105, #125, #156, #173, #179, #239, #243, #245, #340, #352, #357, #358, #371, #441, #462, #482, #494, #525, #547, #591, #593, #595, #622, #646, #653, #660, #671, #677, #700, #765, #797, #836, #866, #875, #897, #960, #971, #1074
- Upstream PRs: PR #108, PR #405, PR #664, PR #763, PR #873
- Platforms: apple
- Behavior: The default is WhenUnlockedThisDeviceOnly. Each option maps one-to-one to kSecAttrAccessible and is validated. Docs state, per level, what survives uninstall and iCloud backup.
- Test: `apple_default_accessibility_is_this_device_only`

## R22. Android backup and reinstall never lose or corrupt data silently

- Upstream issues: #11, #27, #78, #120, #138, #238, #423, #817, #853, #876, #887
- Upstream PRs: none
- Platforms: android
- Behavior: The values file lives in no_backup and is excluded from backup and transfer. Ciphertext found without its Keystore key returns `key_missing`, is kept, and is never deleted. Reinstall starts empty, and the docs say so.
- Test: `restored_ciphertext_without_key_is_kept_and_reported`

## R23. Docs: what each platform protects, and what survives uninstall, backup or sync

- Upstream issues: #98, #228, #322, #413, #463, #523, #550, #613, #667, #738, #742, #795, #917, #1176
- Upstream PRs: PR #367, PR #370, PR #479, PR #538, PR #770, PR #931, PR #1141, PR #1146, PR #1270, PR #1273, PR #1275
- Platforms: typescript, core (docs)
- Behavior: One table per platform: what it protects against, and what survives uninstall, backup and sync. Linux needs a running Secret Service. The web is not a secret store, and the docs say so.
- Test: `docs_platform_table_covers_every_platform`

## R24. Apple locked or background device is a coded error

- Upstream issues: #94, #223, #316, #394, #585, #727, #743
- Upstream PRs: PR #629
- Platforms: apple
- Behavior: errSecInteractionNotAllowed (-25308) returns a coded `locked` error. It never returns null and never hangs. A protected-data availability query is exposed.
- Test: `apple_locked_device_returns_coded_error`

## R25. Apple OSStatus mapping

- Upstream issues: #291, #293, #469, #519, #709, #750, #798, #804, #1104
- Upstream PRs: PR #606, PR #662, PR #710
- Platforms: apple
- Behavior: errSecItemNotFound maps to absent. errSecMissingEntitlement (-34018) returns `entitlement_missing` with guidance. Any other status returns `keychain_error` with the numeric status. No value appears in the message.
- Test: `apple_status_mapping_table`

## R26. Blocking backend calls run off the engine thread

- Upstream issues: #116, #119, #218, #337, #342, #1017
- Upstream PRs: PR #117, PR #536, PR #791, PR #906
- Platforms: core, android, apple, linux, windows
- Behavior: Keystore, biometric, Secret Service, DPAPI and Keychain calls run through nsh_core::blocking, so the UI and engine threads never wait. The web backend stays async, because WebAssembly has no threads.
- Test: `backend_calls_run_off_engine_thread`

## R27. A biometric prompt resolves exactly once per call

- Upstream issues: #1042, #1059, #1125, #1161, #1278
- Upstream PRs: PR #1060, PR #1065, PR #1083, PR #1147, PR #1267, PR #1268, PR #1279
- Platforms: android
- Behavior: Success, cancel, negative button, lockout and retry each resolve the call once. A second callback is ignored and never crashes.
- Test: `biometric_cancel_resolves_once`

## R28. Reads are ordered after writes on one store

- Upstream issues: #119, #218, #337, #342, #343, #457, #566, #592, #716, #744, #1124
- Upstream PRs: PR #117, PR #717, PR #906, PR #1087, PR #1131, PR #1133
- Platforms: core
- Behavior: Operations on one store are ordered by a revision. A read sees the latest completed write for that key. readAll returns every key. No transient null appears during or after a write.
- Test: `read_during_write_never_returns_stale_or_null`

## R29. Web: an undecryptable entry reads as None

- Upstream issues: #381, #869, #1063
- Upstream PRs: PR #782, PR #847, PR #1061, PR #1062
- Platforms: web
- Behavior: A malformed or undecryptable entry reads as None. read and readAll never throw on stored data. A DOMException is caught.
- Test: `web_undecryptable_entry_reads_as_none`

## R30. Web: a secure context is required, and the failure is coded

- Upstream issues: #300, #355, #389, #404, #531, #577, #630, #633, #665, #749, #773, #825, #1016
- Upstream PRs: PR #928
- Platforms: web
- Behavior: Outside a secure context, every call returns `insecure_context`. The docs say the browser may clear site storage at any time.
- Test: `web_insecure_context_returns_coded_error`

## R31. Errors and logs never contain a value

- Upstream issues: #460
- Upstream PRs: PR #217, PR #623
- Platforms: core, all platforms
- Behavior: Display, Debug and logs show the code and the key name only. Platform messages are scrubbed before they are returned.
- Test: `error_text_never_contains_the_value`

## R32. Android native libraries are 16 KB page aligned

- Upstream issues: #988
- Upstream PRs: none
- Platforms: android
- Behavior: Every shipped .so is aligned to 16 KB, as newer Android releases require.
- Test: `android_native_libs_are_16k_aligned`

