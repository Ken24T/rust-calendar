# Google OAuth Setup For Writable Calendar Sync

This guide explains how to configure Google OAuth so Rust Calendar can use
writable Google Calendar sync.

## What You Need

Writable Google CRUD does not use a manually pasted access token.

Rust Calendar uses Google's device authorization flow. In practice, you need:

- a Google Cloud project
- the Google Calendar API enabled for that project
- an OAuth consent screen configured for the project
- a Desktop App OAuth client ID

You then paste the client ID into Rust Calendar and complete the browser sign-in
prompt. Rust Calendar stores the refresh token and refreshes access tokens
automatically.

## Read-Only vs Writable Sync

There are two different Google sync paths in the app:

- Read-only ICS sync: uses a Google Calendar ICS URL and does not require OAuth
- Writable Google API sync: requires OAuth and a linked Google account

If you want create, update, and delete support, you need the writable Google API
path.

## Step 1: Create A Google Cloud Project

1. Open the Google Cloud Console.
2. Choose an existing project or create a new one.
3. Give it a clear name such as `Rust Calendar Local Sync`.

## Step 2: Enable The Google Calendar API

1. In Google Cloud Console, open `APIs & Services`.
2. Open `Library`.
3. Search for `Google Calendar API`.
4. Open it and click `Enable`.

Without this, OAuth can succeed but calendar operations will still fail.

## Step 3: Configure The OAuth Consent Screen

1. Open `APIs & Services`.
2. Open `OAuth consent screen`.
3. Choose the appropriate user type:
   - `External` if you want to use a normal personal Google account
   - `Internal` only if you are using a Google Workspace organisation and know
     that restriction is appropriate
4. Fill in the required app information.

Recommended minimum fields:

- App name: `Rust Calendar`
- User support email: your own email
- Developer contact email: your own email

For a personal/local tool, this is usually enough for testing.

## Step 4: Add Yourself As A Test User If Needed

If your consent screen is in testing mode and the app is external, add the
Google account you plan to use as a test user.

If you skip this, Google may block the sign-in even if the OAuth client is
correct.

## Step 5: Create A Desktop OAuth Client ID

1. Open `APIs & Services`.
2. Open `Credentials`.
3. Click `Create Credentials`.
4. Choose `OAuth client ID`.
5. For application type, choose `Desktop app`.
6. Give it a name such as `Rust Calendar Desktop`.
7. Create the credential.

Google will show:

- a client ID
- a client secret

Rust Calendar needs the client ID. It does not currently ask you to enter the
client secret in the app UI.

The client ID looks like this:

`123456789012-abcdefg1234567890.apps.googleusercontent.com`

That is the value you paste into Rust Calendar.

## Step 6: Link The Account In Rust Calendar

1. Open Rust Calendar.
2. Open `Settings`.
3. Go to the calendar sync section.
4. Paste the Desktop OAuth client ID into the `OAuth client ID` field.
5. Click `Connect / Reconnect`.

The app will start Google's device flow and open a browser page if possible.

## Step 7: Complete Google's Device Login Prompt

When you click `Connect / Reconnect`, Google will ask you to sign in and approve
access.

Approve the requested calendar permissions for the Google account you want Rust
Calendar to use.

When the device flow completes successfully:

- the account email should appear as connected in the app
- Rust Calendar stores the refresh token locally
- the app can request fresh access tokens automatically later

## Step 8: Use A Writable Google Source

OAuth alone is not enough. You also need to configure or use a calendar source
that is set up for writable Google sync.

Operationally, the expected end state is:

- a connected Google account in settings
- a writable Google calendar source
- successful sync runs in the recent sync history

## What Rust Calendar Stores

Rust Calendar stores the Google auth state locally in its application database,
including:

- the OAuth client ID
- the account email
- the access token
- the refresh token
- expiry metadata

These are stored locally on your machine, not in a remote Rust Calendar service.

## Common Problems

### The OAuth Client ID Is Rejected

Check that you created a `Desktop app` OAuth client and pasted the client ID,
not some other identifier.

The app expects a client ID ending in `.apps.googleusercontent.com`.

### Google Sign-In Opens But Authorization Fails

Check:

1. the Google Calendar API is enabled
2. the OAuth consent screen is configured
3. your Google account is added as a test user if the app is in testing mode

### The App Connects But Later Stops Syncing

Use the calendar sync settings section to inspect:

- last status
- recent sync runs
- outbound queue failures

If the token has expired, use `Refresh Token`.

If the refresh token is revoked or invalid, reconnect the account.

### You Only Have ICS Read-Only Sync Working

That is expected if you only configured an ICS URL.

To get CRUD, you must also complete Google OAuth and use the writable Google API
sync path.

## Security Notes

- Do not publish your personal desktop client ID and project details unless you
  mean to share that Google Cloud setup
- Do not paste raw access tokens into chat, issues, or source control
- If you believe the refresh token is compromised, revoke access in your Google
  account and reconnect from scratch

## Related Docs

- [GOOGLE_CALENDAR_STAGE2_OPERATIONS.md](GOOGLE_CALENDAR_STAGE2_OPERATIONS.md)
- [USER_GUIDE.md](USER_GUIDE.md)