/** When an item can be read. The default is `unlockedThisDevice`. */
export type Accessibility =
  | "passcode"
  | "unlocked"
  | "unlockedThisDevice"
  | "firstUnlock"
  | "firstUnlockThisDevice";

/** Access-control flags an item is written with. */
export type AccessControlFlag =
  | "devicePasscode"
  | "biometryAny"
  | "biometryCurrentSet"
  | "userPresence"
  | "watch"
  | "or"
  | "and"
  | "applicationPassword"
  | "privateKeyUsage";

/** Options for iOS: the Keychain. */
export interface IOSOptions {
  /** The service items are filed under. Default `nsh_secure_storage`. */
  accountName?: string;
  /** The access group items are shared through. */
  groupId?: string;
  accessibility?: Accessibility;
  /** Whether the item syncs through iCloud Keychain. */
  synchronizable?: boolean;
  accessControlFlags?: AccessControlFlag[];
  /** `fail` and `skip` never show a system dialog. */
  authenticationUI?: "allow" | "fail" | "skip";
  /** The text an authentication prompt shows. */
  operationPrompt?: string;
}

/** Options for macOS: the Keychain, with a file keychain as an option. */
export interface MacOSOptions extends IOSOptions {
  /** Default `true`. `false` uses the file keychain at `keychainPath`. */
  usesDataProtectionKeychain?: boolean;
  /** A keychain file, used when the data protection keychain is off. */
  keychainPath?: string;
}

/** Options for Android. The Android backend is built by another part of the kit. */
export interface AndroidOptions {
  /**
   * Whether a permanently invalidated key may reset the store. Default `false`: a silent
   * reset loses the user's data. Only an explicit, permanent signal can reset it.
   */
  resetOnError?: boolean;
}

/** Options for Linux. None yet: the Linux backend is pending. */
export interface LinuxOptions {}

/** Options for Windows. None yet: the Windows backend is pending. */
export interface WindowsOptions {}

/** Options for the web. None yet: the web backend is pending. */
export interface WebOptions {}

/** The options of a store, one object per platform. Each backend reads its own. */
export interface SecureStorageOptions {
  iOptions?: IOSOptions;
  mOptions?: MacOSOptions;
  aOptions?: AndroidOptions;
  lOptions?: LinuxOptions;
  wOptions?: WindowsOptions;
  webOptions?: WebOptions;
}
