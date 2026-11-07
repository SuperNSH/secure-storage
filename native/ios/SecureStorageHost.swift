// The iOS half of `nsh-secure-storage/host`: the `secureStorage.*` handlers of an embedding app,
// on the Keychain. Writes update an item in place and never delete it first; a failed read or
// write never removes a value, and nothing is written to disk outside the Keychain.

import Foundation
import LocalAuthentication
import Security
import UIKit

/// A failure the handlers reject with: `code` is an `ERR_SECURE_STORAGE_*` code, `detail` says
/// what failed. Neither holds a value.
public struct SecureStorageHostError: Error, LocalizedError {
    public let code: String
    public let detail: String

    public var errorDescription: String? { "\(code): \(detail)" }
}

/// The handlers of `nsh-secure-storage/host`. Register each name with `NshAppView.handle`:
///
///     for name in SecureStorageHost.names {
///         app.handle(name) { data in try SecureStorageHost.handle(name, data: data) }
///     }
public enum SecureStorageHost {
    /// Every handler name the TSX backend calls.
    public static let names = [
        "secureStorage.read", "secureStorage.write", "secureStorage.delete",
        "secureStorage.containsKey", "secureStorage.readAll", "secureStorage.deleteAll",
        "secureStorage.protectedDataAvailable",
    ]

    /// Runs the handler `name` with the call's `data`, and returns its JSON answer.
    public static func handle(_ name: String, data: Any?) throws -> Any? {
        let call = data as? [String: Any] ?? [:]
        let store = Store(options: call["options"] as? [String: Any] ?? [:])
        switch name {
        case "secureStorage.read":
            let key = try keyOf(call)
            return ["value": try store.read(key) as Any? ?? NSNull()]
        case "secureStorage.write":
            try store.write(try keyOf(call), value: try valueOf(call))
            return nil
        case "secureStorage.delete":
            try store.delete(try keyOf(call))
            return nil
        case "secureStorage.containsKey":
            return ["present": try store.read(try keyOf(call)) != nil]
        case "secureStorage.readAll":
            return ["entries": try store.readAll().map { ["key": $0.key, "value": $0.value] }]
        case "secureStorage.deleteAll":
            try store.deleteAll()
            return nil
        case "secureStorage.protectedDataAvailable":
            return ["available": UIApplication.shared.isProtectedDataAvailable]
        default:
            throw SecureStorageHostError(code: "ERR_SECURE_STORAGE_NOT_AVAILABLE", detail: "unknown handler")
        }
    }

    private static func keyOf(_ call: [String: Any]) throws -> String {
        guard let key = call["key"] as? String, !key.isEmpty, !key.contains("\u{0}") else {
            throw SecureStorageHostError(code: "ERR_SECURE_STORAGE_INVALID_KEY", detail: "the key is empty or has a NUL")
        }
        return key
    }

    private static func valueOf(_ call: [String: Any]) throws -> String {
        guard let value = call["value"] as? String else {
            throw SecureStorageHostError(code: "ERR_SECURE_STORAGE_BACKEND", detail: "the value is not a string")
        }
        return value
    }
}

/// One Keychain service, as the store's options name it.
private struct Store {
    let service: String
    let accessible: CFString
    let synchronizable: Bool

    init(options: [String: Any]) {
        let ios = options["ios"] as? [String: Any] ?? [:]
        service = ios["accountName"] as? String ?? "nsh_secure_storage"
        synchronizable = ios["synchronizable"] as? Bool ?? false
        switch ios["accessibility"] as? String ?? "unlockedThisDevice" {
        case "passcode": accessible = kSecAttrAccessibleWhenPasscodeSetThisDeviceOnly
        case "unlocked": accessible = kSecAttrAccessibleWhenUnlocked
        case "firstUnlock": accessible = kSecAttrAccessibleAfterFirstUnlock
        case "firstUnlockThisDevice": accessible = kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
        default: accessible = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        }
    }

    /// The query that finds one item, in every accessibility level and both sync states.
    private func query(_ key: String) -> [String: Any] {
        [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrAccount as String: key,
            kSecAttrSynchronizable as String: kSecAttrSynchronizableAny,
        ]
    }

    func read(_ key: String) throws -> String? {
        var query = query(key)
        query[kSecReturnData as String] = true
        query[kSecMatchLimit as String] = kSecMatchLimitOne
        query[kSecUseAuthenticationContext as String] = noPrompt()
        var item: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &item)
        if status == errSecItemNotFound { return nil }
        try check(status, key)
        guard let data = item as? Data, let value = String(data: data, encoding: .utf8) else {
            throw SecureStorageHostError(code: "ERR_SECURE_STORAGE_CORRUPT", detail: "the item is not UTF-8")
        }
        return value
    }

    /// Updates the item in place when one exists, and adds it otherwise.
    func write(_ key: String, value: String) throws {
        let data = Data(value.utf8)
        let update: [String: Any] = [kSecValueData as String: data, kSecAttrAccessible as String: accessible]
        let updated = SecItemUpdate(query(key) as CFDictionary, update as CFDictionary)
        if updated == errSecSuccess { return }
        if updated != errSecItemNotFound { try check(updated, key) }
        var add = query(key)
        add[kSecValueData as String] = data
        add[kSecAttrAccessible as String] = accessible
        add[kSecAttrSynchronizable as String] = synchronizable
        try check(SecItemAdd(add as CFDictionary, nil), key)
    }

    func delete(_ key: String) throws {
        let status = SecItemDelete(query(key) as CFDictionary)
        if status != errSecItemNotFound { try check(status, key) }
    }

    func readAll() throws -> [(key: String, value: String)] {
        var query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: service,
            kSecAttrSynchronizable as String: kSecAttrSynchronizableAny,
            kSecReturnAttributes as String: true,
            kSecMatchLimit as String: kSecMatchLimitAll,
        ]
        query[kSecUseAuthenticationContext as String] = noPrompt()
        var items: CFTypeRef?
        let status = SecItemCopyMatching(query as CFDictionary, &items)
        if status == errSecItemNotFound { return [] }
        try check(status, "")
        let keys = (items as? [[String: Any]] ?? []).compactMap { $0[kSecAttrAccount as String] as? String }
        return try keys.sorted().compactMap { key in
            guard let value = try read(key) else { return nil }
            return (key, value)
        }
    }

    func deleteAll() throws {
        for entry in try readAll() { try delete(entry.key) }
    }

    /// A context that never shows a prompt: a read needing one fails with `errSecInteractionNotAllowed`.
    private func noPrompt() -> LAContext {
        let context = LAContext()
        context.interactionNotAllowed = true
        return context
    }

    /// Maps a Security status to a code. The status number is kept; no value is.
    private func check(_ status: OSStatus, _ key: String) throws {
        guard status != errSecSuccess else { return }
        let code: String
        switch status {
        case errSecInteractionNotAllowed: code = "ERR_SECURE_STORAGE_LOCKED"
        case errSecAuthFailed: code = "ERR_SECURE_STORAGE_AUTH_FAILED"
        case errSecUserCanceled: code = "ERR_SECURE_STORAGE_AUTH_CANCELLED"
        case errSecMissingEntitlement: code = "ERR_SECURE_STORAGE_MISSING_ENTITLEMENT"
        default: code = "ERR_SECURE_STORAGE_BACKEND"
        }
        let named = key.isEmpty ? "" : " (key \"\(key)\")"
        throw SecureStorageHostError(code: code, detail: "Security status \(status)\(named)")
    }
}
