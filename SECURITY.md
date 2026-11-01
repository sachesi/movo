# Security

Please report a vulnerability privately rather than in a public issue: through
[a private advisory](https://github.com/sachesi/movo/security/advisories/new) on GitHub,
or by mail to sachesi <xsachesi@pm.me>. Say what you found, how to reproduce it and which
version you ran; a fix is worked out with you before anything is published.

Only the latest release gets fixes.

## What counts

Movo signs in to an account on `hdrzk.org` and reads whatever that site sends back. The
parts where a mistake matters most:

- The session. Cookies are kept in the Secret Service keyring on Linux and a file
  encrypted with the Android Keystore on Android, and the password is never stored (see
  [docs/authentication.md](docs/authentication.md)). A session reaching a settings file, a
  log, an error message or a command line, reaching any host but the provider, staying
  after logout, or carrying data over from one account to another is a vulnerability.
- What the site's responses control. Stream and subtitle addresses, titles and other text
  reach the desktop player as arguments. A response that makes Movo run another program,
  pass an option to the player, or open anything but a media address is a vulnerability.
- The player's control socket, kept in the user's runtime directory. A way for another
  user to reach it is a vulnerability.
- The Android app's entry points. It takes links to `https://hdrzk.org` from other apps. A
  link or intent that does anything more than open a page in the app is a vulnerability.

Movo is an unofficial client, not affiliated with the site. The site's own security, the
content it serves and flaws in mpv or another player you choose are not ours to fix. A
page that stops loading because the site's markup changed is a bug; please file it as an
ordinary issue.
