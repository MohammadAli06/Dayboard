# Security

Dayboard is an early-development local application, not a hardened multi-user service. SQLite and its audit log contain unencrypted personal information. Protect your OS account and backups.

Do not post real databases, access tokens or sensitive task notes in public issues. If the repository owner enables GitHub private vulnerability reporting, use **Security → Report a vulnerability**. Otherwise ask the maintainer for a private reporting channel without disclosing exploit details or private data publicly.

The desktop app has a restrictive content security policy and minimal Tauri window permissions. There is no remote command bridge in this release. Any future bridge must be separately specified, default-off, loopback-only and authenticated.
