# Rust Calendar — User Guide

This guide covers how to use Rust Calendar, a desktop calendar application for
Linux and Windows.

## Getting Started

When you first launch Rust Calendar, you'll see a month view with today's date
highlighted. The sidebar on the left shows a mini calendar, today's events, and
upcoming events.

Your data is stored locally in a SQLite database — there are no accounts,
logins, or cloud services. Everything stays on your machine.

## Calendar Views

Switch between views using the **View → Calendar Views** menu or single-key
shortcuts (when no dialog is open):

- **Day** (`D`) — hourly schedule for a single day
- **Week** (`W`) — full 7-day view with time slots
- **Work Week** (`K`) — Monday through Friday only
- **Month** (`M`) — traditional monthly grid

Navigate between periods with `←` and `→` arrow keys, or press `Ctrl+T` to
jump back to today.

## Creating Events

Press `Ctrl+N` or use **Events → New Event…** to open the event dialog.

### Required fields

- **Title** — the event name (required; the Save button is disabled until you
  enter a title)

### Optional fields

- **Location** — where the event takes place (includes a "Open in Google Maps"
  button when filled in)
- **Category** — select from your categories (e.g. Work, Personal, Birthday);
  each category has a colour
- **Description** — free-text notes
- **Date and time** — start and end dates with time pickers; tick **All day** to
  hide the time fields
- **Colour** — custom colour via hex input, colour picker, or preset buttons
  (Blue, Green, Red, Yellow, Purple, Pink)
- **Recurrence** — tick **Repeat this event** to set up repeating events:
  - Frequency: Daily, Weekly, Monthly, Yearly
  - Interval: e.g. "every 2 weeks"
  - Pattern: for Monthly/Yearly — None, First Day, Last Day
  - Days of the week: toggle individual days
  - End condition: after N occurrences or by a specific date
- **Create countdown card** — tick this to create a desktop countdown widget for
  the event (only available for future events)

The dialog will show a warning banner if the event overlaps with existing events.

### Editing events

Double-click an event in any view to open it for editing. For past events, the
date and time fields are read-only.

### Deleting events

Open an event for editing and click **Delete**. You'll be asked to confirm.

## Recurring Events

Rust Calendar supports these recurrence patterns:

- **Daily** — every N days
- **Weekly** — every N weeks on selected days
- **Fortnightly** — every 2 weeks
- **Monthly** — every N months (same date, first day, or last day of month)
- **Quarterly** — every 3 months
- **Yearly** — every N years

When editing a recurring event, you can modify a single occurrence or the entire
series.

## Sidebar

Toggle the sidebar with `Ctrl+\` or **View → Show Sidebar**.

The sidebar has three sections:

1. **Mini Calendar** — click any date to navigate the main view. Use the arrows
   to move between months. Click the month/year header to jump to today.
2. **Today's Events** — up to 5 events for the selected day. Click an event
   title to navigate to it.
3. **Upcoming Events** — the next 10 events within 30 days.

The sidebar is resizable by dragging its edge (150–300 px). You can move it to
the right side in **Edit → Settings → View → Position on right side**.

## All-Day Event Ribbon

Events that span multiple days appear in a ribbon at the top of Day, Week, and
Work Week views. Toggle this with **View → Show All-Day Events Ribbon**.

The ribbon has three display modes (compact, expanded, auto) and shows progress
indicators for events currently in progress.

## Countdown Timers

Create a countdown timer for any future event by ticking **Create countdown
card** in the event dialog.

Countdown cards appear as small floating windows showing the time remaining
until an event starts. You can:

- Display them as **individual windows** (each card is a separate, always-on-top
  window) or in a **combined container** (all cards in one resizable window) —
  switch via **View → Countdown Cards → Display mode**
- Customise fonts, colours, and dimensions per card
- Reset card positions via **View → Countdown Cards → Reset Card Positions**

### Creating from a Context Menu

Right-click any future event in a calendar view and choose **⏱ Create
Countdown**. If you have multiple countdown categories, a submenu appears
listing each category so you can choose the target container.

### Category Containers

Countdown cards can be organised into named categories (containers). Open
**Edit → Manage Countdown Categories…** to create, edit, or delete categories.

Each container has:

- A **collapsible header** (▶/▼ toggle) with a card count badge
- A **sort mode** switch — 📅 Date (by event start) or ✋ Manual
- A **➕ quick-add** button to create a new card directly in that container
- A **template** selection and **layout orientation** — configured in the
  "Card Defaults" section when editing a category

Cards can be **dragged between containers** to re-categorise them.

### Card Templates

Reusable card templates define colours, fonts, and default card dimensions.
Open **Edit → Manage Card Templates…** to create, edit, or delete templates.

### Visual Inheritance

Card visuals follow a four-tier model:

1. **Global defaults** — base settings for all cards
2. **Template** — reusable set of colours, fonts, and default dimensions;
   selected per-category via the Category Manager
3. **Category overrides** — card dimensions can be overridden per-container
4. **Per-card overrides** — configured in the individual card's settings panel

## Drag and Drop

In Day, Week, and Work Week views, you can **drag events** to move them to a
different time slot or day. The event's duration is preserved.

You can also **drag `.ics` files** from your file manager into the calendar
window to import events.

## Categories

Manage categories via **Edit → Manage Categories…** or in the event dialog's
category dropdown.

The app ships with default categories (Work, Personal, Birthday, Holiday,
Meeting, Deadline). You can add, rename, recolour, or delete categories.

Filter the calendar to show only specific categories via **View → Filter by
Category**.

## Templates

Save frequently-used event configurations as templates:

1. Create an event with the settings you want
2. Go to **Events → Templates → Manage Templates…**
3. Templates can be applied when creating new events via **Events → Templates**

## Import and Export

### iCalendar (.ics)

- **Import**: **Events → Import Event…** or drag a `.ics` file into the window
- **Export**: **Events → Export Events →** with options for filtered events, all
  events, or a date range

### PDF

- **Export**: **File → Export to PDF →** with options for Month View, Week View,
  or All Events

### Backup

- **Automatic backup**: a backup is created automatically each time the
  application starts. The five most recent automatic backups are kept; older ones
  are removed automatically.
- **Create backup**: `Ctrl+B` or **File → Backup Database…** — creates a
  timestamped copy of your database. Manual backups are not subject to automatic
  cleanup.
- **Manage backups**: **File → Manage Backups…** — view, restore, or delete
  backups
- **Restore**: restoring a backup automatically creates a safety copy of your
  current database before overwriting. A restart is required after restoring.
- **Location**: backups are stored in your system's app data directory
  (`%AppData%\rust-calendar\backups\` on Windows, `~/.local/share/rust-calendar/backups/`
  on Linux)

## Themes

Rust Calendar includes light and dark themes.

- **Switch themes**: **View → Themes** and select a theme
- **Create custom themes**: **Edit → Manage Themes…** opens the theme editor
- **System theme**: enable **Use system theme** in Settings to follow your OS
  light/dark preference

## System Tray

- **Enable**: go to **Edit → Settings → View** and tick "Minimise to system
  tray on close"
- **Behaviour**: when enabled, closing the main window hides it to the system
  tray instead of exiting. Countdown cards remain visible and responsive.
- **Restore**: click the tray icon or right-click and select "Show Calendar"
- **Exit**: right-click the tray icon and select "Exit", or use **File → Exit**
  (always exits regardless of the tray setting)
- **Linux note**: requires a desktop environment with system tray support (KDE,
  XFCE, MATE, Cinnamon, LXQt). On GNOME, install the
  [AppIndicator extension](https://extensions.gnome.org/extension/615/appindicator-support/).
  If no tray host is detected, the setting is automatically disabled.

## Settings

Open Settings with `Ctrl+S` or **Edit → Settings**.

### Calendar

- **First day of week** — which day starts the week (default: Monday)

### Work Week

- **First day** / **Last day** — define your working days

### Time

- **Time format** — 12-hour or 24-hour
- **Date format** — DD/MM/YYYY, MM/DD/YYYY, or YYYY-MM-DD
- **Default event duration** — 15, 30, 45, 60, 90, or 120 minutes
- **Default event start time** — HH:MM

### View

- **Default view** — which view to show on startup
- **Show sidebar** and sidebar position
- **Show week numbers** — ISO week numbers on calendar views
- **Show ribbon** — all-day events ribbon

### Card (Countdown)

- **Default card width/height** — default size for new countdown cards
- **Auto-create countdown cards on ICS import**
- **Open event dialog when importing/dragging ICS files**

Card visual templates are managed via **Edit → Manage Card Templates…** and
assigned per-category in **Edit → Manage Countdown Categories…**.

### Calendar Sync

- Google Calendar sync configuration (ICS feed URL, sync interval, startup
  delay)

## Keyboard Shortcuts

- `Ctrl+N` — new event
- `Ctrl+F` — search events
- `Ctrl+T` — go to today
- `Ctrl+S` — open settings
- `Ctrl+B` — backup database
- `Ctrl+Z` — undo
- `Ctrl+Y` or `Ctrl+Shift+Z` — redo
- `Ctrl+\` — toggle sidebar
- `D` — Day view
- `W` — Week view
- `K` — Work Week view
- `M` — Month view
- `Left` / `Right` — previous / next period
- `Up` / `Down` — navigate up / down (contextual)
- `Escape` — close the current dialog

Single-key shortcuts (`D`, `W`, `K`, `M`, arrows) are only active when no
dialog is open and you are not typing in a text field.

## Data Storage

All data is stored locally:

- **Database**: SQLite file in your system's app data directory
- **Themes**: TOML files in `assets/themes/` (built-in) or in the database
  (custom themes)
- **Backups**: timestamped database copies in the backups subdirectory

No data is sent to any server. The optional Google Calendar sync feature only
reads from public ICS feed URLs that you explicitly configure.
