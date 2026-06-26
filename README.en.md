# Microsoft Email Manager Desktop

[Simplified Chinese](README.md) | [English](README.en.md)

A local desktop application for managing Microsoft email accounts and messages. It is rebuilt from [Microsoft-Email-Manager](https://github.com/Maishan-Inc/Microsoft-Email-Manager), the FastAPI web version, as a standalone **Tauri + Rust + Svelte** desktop app.

## Why This Stack

The stack is chosen around five core requirements:

| Requirement | Implementation |
|---|---|
| **Keep data local** | All data is stored in an encrypted SQLite database (`mem-desktop.db`) under the system application data directory. No backend server is used. |
| **Encrypt account credentials** | `refresh_token` / `client_id` are encrypted with **AES-256-GCM**. The key is derived from the master password with **Argon2id**. The master password is never stored and cannot be recovered. |
| **Support export** | Accounts can be exported as JSON / CSV. Exports can optionally include credentials, which forces encrypted export for the whole file. |
| **Low memory usage** | Tauri uses the system WebView instead of bundling Chromium. The measured main process uses about **47 MB** of memory, compared with Electron's typical 120 MB+. The Rust backend has no GC. |
| **Simple cross-platform packaging** | Tauri supports Windows / macOS / Linux packaging natively. HTTP uses rustls, avoiding OpenSSL. Only Linux IMAP uses native-tls. |

## Features

- **Left sidebar app shell**: dashboard / email account management / add email / category editor / settings + lock, with light/dark themes and Chinese/English switching.
- **First-run onboarding wizard**: intro animation -> user agreement that must be scrolled to the bottom -> master password setup -> optional 2FA -> authentication mode -> recovery mnemonic generation/download/hidden verification -> first-use tutorial.
- **Master password unlock** using the key-wrapping model described in the security model. Optional **TOTP two-factor authentication**. Forgotten passwords can be reset with the **recovery mnemonic**.
- **Dashboard**: email account count, account health, emails received today, and recent activity.
- **Account management**: horizontal list, click to view emails, add a single account after connection testing, bulk import, and delete.
- **Per-account notifications**: enable system notifications for new email per account. Enabled accounts are refreshed by background polling.
- **Bulk import with multiple formats**:
  - IMAP: `email----refresh token----client ID`, compatible with the legacy `email----placeholder password----refresh token----client ID` format.
  - Graph: `email----password----client_id----token`.
- **Two mail access methods**: **IMAP** with OAuth2 + XOAUTH2, and **Microsoft Graph API**.
- **Inbox / junk / all mail** views, with message list and detail view. Plain text / HTML content is rendered in a safe sandbox.
- **Account health checks** through OAuth refresh and protocol probing.
- **JSON / CSV export** with optional credentials and encryption.
- **Category / tag editor**.

## Architecture

```text
src/                      Frontend (Svelte 5 + TS + Vite)
├─ App.svelte             App shell: sidebar + view routing + onboarding gate
├─ components/
│  ├─ Sidebar.svelte      Left navigation + lock
│  ├─ Dashboard.svelte    Dashboard: account count / health / today's mail / recent activity
│  ├─ Accounts.svelte     Account rows + notification toggle + email view entry
│  ├─ AddEmail.svelte     Single add / bulk import
│  ├─ Categories.svelte   Category / tag editor
│  ├─ Settings.svelte     Language / theme / background refresh / export / about
│  ├─ Emails.svelte       Message list + detail view
│  ├─ Unlock.svelte       Unlock / 2FA / mnemonic recovery
│  └─ onboarding/OnboardingWizard.svelte  First-run onboarding wizard
├─ lib/{api,types,toast,i18n,theme}  invoke wrapper / types / toast / Chinese-English i18n / theme
src-tauri/src/            Backend (Rust)
├─ crypto.rs              Argon2id derivation + AES-256-GCM + DEK key wrapping
├─ security.rs            TOTP (RFC 6238) / Base32 / QR code / BIP39 mnemonic
├─ db.rs                  rusqlite (bundled) models and migrations, including mail_activity
├─ state.rs               Unlocked Vault + pending 2FA state; zeroized on lock
├─ background.rs          Background new-mail polling + system notifications
├─ accounts_auth.rs       Microsoft OAuth2 token refresh
├─ accounts.rs            Bulk import parsing / connection testing
├─ imap_client.rs         IMAP XOAUTH2 mail fetching with spawn_blocking
├─ graph.rs               Graph API mail fetching
├─ export.rs              JSON / CSV export
└─ commands.rs            #[tauri::command] API exposed to the frontend
```

## Security Model

- **Key wrapping**: a random 32-byte data encryption key (DEK) encrypts account fields with AES-256-GCM using `base64(nonce(12) || ciphertext+tag)`.
- The DEK is wrapped by multiple key encryption keys (KEKs) on disk:
  - Master password -> Argon2id (64 MiB / 3 iterations) -> `KEK_pw` -> `dek_wrapped_pw`.
  - Optional recovery mnemonic (BIP39) -> Argon2id -> `KEK_mn` -> `dek_wrapped_mn`.
  - GCM authentication tags validate unwrap attempts. Failure means the password or mnemonic is wrong.
- **Optional TOTP two-factor authentication**: the secret is encrypted with the DEK. When enabled, unlock requires master password -> DEK unwrap -> 6-digit TOTP verification.
- While unlocked, the DEK stays resident in memory. It is zeroized on lock or exit with `zeroize`.
- **Recovery**: if the master password is forgotten, the recovery mnemonic can unwrap the DEK and reset the password. Without a configured mnemonic, recovery is impossible. This is the tradeoff of local encryption.
- Encrypted export files start with `MEMENC1` and can be decrypted/restored inside this app.

## Development

Requirements: Node >= 18, pnpm, Rust >= 1.77.

```bash
pnpm install
pnpm tauri dev      # development mode with hot reload
```

Type checking / frontend build:

```bash
pnpm check
pnpm build
```

## Cross-Platform Packaging

```bash
pnpm tauri build
```

Build artifacts are placed under `src-tauri/target/release/bundle/`.

Platform prerequisites:

| Platform | Prerequisites |
|---|---|
| **Windows** | WebView2 Runtime, already built into many Windows 10/11 systems. Produces `.msi` / `.exe (NSIS)`. |
| **macOS** | Xcode Command Line Tools. Produces `.app` / `.dmg`. Native TLS uses SecureTransport, so OpenSSL is not required. |
| **Linux** | `webkit2gtk`, `libssl-dev` for IMAP native-tls. Produces `.deb` / `.AppImage` / `.rpm`. |

> HTTP requests for token refresh / Graph use rustls and do not require OpenSSL. Only IMAP native-tls on Linux needs `libssl-dev`.

### GitHub Actions Cross-Platform Builds (Optional)

The official `tauri-apps/tauri-action` can produce installers for Windows, macOS, and Linux with a `windows-latest` / `macos-latest` / `ubuntu-latest` matrix.

## Data Location

- Windows: `%APPDATA%\com.maishan.mem.desktop\mem-desktop.db`
- macOS: `~/Library/Application Support/com.maishan.mem.desktop/mem-desktop.db`
- Linux: `~/.local/share/com.maishan.mem.desktop/mem-desktop.db`

## Open Source Notice

The source project is licensed under CC BY-NC 4.0. This desktop version follows the same license and is intended for learning, research, and personal use.

[linux.do](https://linux.do)
