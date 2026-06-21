# Changelog

## v0.1.1-cli-alpha

- Added account login/password authorization through `POST /auth/account/login`.
- Reworked `floppa-cli login` to choose between Telegram and account login/password.
- Added `floppa-cli login --method account --login <login>` and kept `floppa-cli login-account` as a direct account-login path.
- Added tests for account login request shape, credential handling via environment variable, token persistence, and CLI method parsing.
