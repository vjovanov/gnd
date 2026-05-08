# FS-001-login: A player can log in

## 1. Inputs

- username
- password

## 2. Success path

Returns a session token.

## 3. Failure modes

### 3.1 Rate limiting

More than 5 failures locks the IP.

### 3.2 Locked account

Locked accounts cannot log in.
