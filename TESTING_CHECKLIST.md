# Comprehensive Testing Checklist - Egui Refactor

## Test Date: November 10, 2025

## 1. Basic Application Startup
- [ ] Application launches without errors
- [ ] Default view displays (Month)
- [ ] Database initializes properly
- [ ] Settings load correctly
- [ ] Theme applies correctly (check default)

## 2. View Navigation
### Menu-Based View Switching
- [ ] View → Day (switches to day view)
- [ ] View → Week (switches to week view)
- [ ] View → Work Week (switches to work week view)
- [ ] View → Month (switches to month view)
- [ ] View → Quarter (switches to quarter view)

### Navigation Buttons
- [ ] "Previous" button moves backward in time
- [ ] "Today" button returns to current date
- [ ] "Next" button moves forward in time
- [ ] Navigation works correctly in each view type

## 3. Month View Testing
### Display
- [ ] Current month displays correctly
- [ ] Day of week headers visible (Sun-Sat)
- [ ] Days properly laid out in 6x7 grid
- [ ] Today is highlighted
- [ ] Weekends have different background color
- [ ] Month/year header shows correct date

### Events
- [ ] Existing events display in correct day cells
- [ ] Event color bars show correctly
- [ ] Event titles are readable
- [ ] "+N more" appears when >3 events
- [ ] Events are clickable to edit
- [ ] Clicking event opens dialog with correct data

### Event Creation
- [ ] Single-click on empty day opens event dialog
- [ ] Dialog defaults to clicked date
- [ ] Single-click sets FREQ=DAILY
- [ ] Double-click on empty day opens event dialog
- [ ] Double-click sets FREQ=MONTHLY

## 4. Quarter View Testing
### Display
- [ ] Three months display side by side
- [ ] Correct quarter shown based on current date
- [ ] Each mini-month has 6x7 grid
- [ ] Day headers visible (S/M/T/W/T/F/S)
- [ ] Today is highlighted
- [ ] Navigation works (previous/next quarter)

### Interaction
- [ ] Single-click opens event dialog
- [ ] Single-click sets FREQ=DAILY
- [ ] Double-click opens event dialog
- [ ] Double-click sets FREQ=MONTHLY;INTERVAL=3

## 5. Day View Testing
### Display
- [ ] Date header shows correct day
- [ ] 24-hour time grid visible
- [ ] Time slot interval respects settings
- [ ] Hour boundaries marked with thicker lines
- [ ] Time labels (HH:00) visible on left

### Events
- [ ] Events display in correct time slots
- [ ] Event color bars show correctly
- [ ] Event titles visible
- [ ] Event times displayed
- [ ] Multiple events in same slot handled
- [ ] Events span multiple slots if long duration

### Interaction
- [ ] Hover shows highlight on time slots
- [ ] Single-click opens event dialog with correct time
- [ ] Double-click opens event dialog with correct time
- [ ] Both create FREQ=DAILY events
- [ ] Scrolling works for full 24-hour view

## 6. Week View Testing
### Display
- [ ] 7 days (Sun-Sat) display as columns
- [ ] Day headers show day name and date
- [ ] Today column is highlighted
- [ ] Time grid spans all 7 days
- [ ] Weekend columns have different background
- [ ] Vertical lines separate days
- [ ] Horizontal lines separate time slots

### Events
- [ ] Events display in correct day/time cells
- [ ] Event colors show correctly
- [ ] Titles truncated appropriately
- [ ] Events clickable (future enhancement)

### Interaction
- [ ] Click opens event dialog with correct day/time
- [ ] Single-click sets FREQ=DAILY
- [ ] Double-click sets FREQ=WEEKLY
- [ ] Scrolling works

## 7. Work Week View Testing
### Display
- [ ] Only work days display (default Mon-Fri)
- [ ] Day headers show correct days
- [ ] Today highlighting works on work days
- [ ] Time grid matches settings

### Settings Integration
- [ ] First day of work week setting respected
- [ ] Last day of work week setting respected
- [ ] Non-contiguous work weeks handled (e.g., Thu-Mon)

### Interaction
- [ ] Click opens event dialog
- [ ] Single-click sets FREQ=DAILY
- [ ] Double-click sets FREQ=WEEKLY
- [ ] Scrolling works

## 8. Event Dialog - Create New Event
### Opening
- [ ] Opens from File → Events → New Event
- [ ] Opens from clicking day cells
- [ ] Opens from double-clicking day cells
- [ ] Dialog centered on screen
- [ ] ESC key closes dialog

### Basic Fields
- [ ] Title field accepts text
- [ ] Location field accepts text
- [ ] Category field accepts text
- [ ] Description multiline field accepts text

### Date/Time
- [ ] Date displays correctly
- [ ] "Previous Day" button works
- [ ] "Today" button works
- [ ] "Next Day" button works
- [ ] All-day checkbox toggles
- [ ] Start time picker works (hour/minute)
- [ ] End time picker works (hour/minute)
- [ ] Time pickers disabled when all-day checked

### Color
- [ ] Color hex field accepts input
- [ ] Color preview displays
- [ ] Preset color buttons work (Blue, Green, Red, Yellow, Purple, Pink)
- [ ] Color picker dialog opens and works
- [ ] Color updates when changed

### Recurrence - Basic
- [ ] "Repeat this event" checkbox toggles recurrence section
- [ ] Frequency dropdown works (Daily/Weekly/Monthly/Yearly)
- [ ] Interval spinner works (1-999)
- [ ] Interval label changes based on frequency

### Recurrence - BYDAY
- [ ] BYDAY option shows for Weekly frequency
- [ ] BYDAY option shows for Monthly frequency
- [ ] "Repeat on specific days" checkbox works
- [ ] All 7 day checkboxes work (Sun-Sat)
- [ ] Multiple days can be selected

### Recurrence - End Conditions
- [ ] "Never" radio button works (default)
- [ ] "After N occurrences" radio button works
- [ ] Occurrence count spinner works (1-999)
- [ ] "Until date" radio button works
- [ ] Until date displays correctly

### Validation
- [ ] Empty title shows error
- [ ] End time before start time shows error
- [ ] No BYDAY days selected shows error (when enabled)
- [ ] Invalid color format shows warning
- [ ] Error message displays at top in red

### Save/Cancel
- [ ] Cancel button closes dialog without saving
- [ ] Save button validates before saving
- [ ] Save button creates event in database
- [ ] Saved event appears in views
- [ ] Dialog closes after successful save

## 9. Event Dialog - Edit Existing Event
### Opening
- [ ] Click on event in month view opens dialog
- [ ] All event fields populated correctly
- [ ] Title matches
- [ ] Description matches
- [ ] Location matches
- [ ] Category matches
- [ ] Date/time matches
- [ ] All-day setting matches
- [ ] Color matches
- [ ] Recurrence settings match (if recurring)

### Editing
- [ ] All fields can be modified
- [ ] Changes are editable
- [ ] Validation still applies

### Save/Delete
- [ ] Save button updates event in database
- [ ] Updated event displays in views
- [ ] Delete button shows in red
- [ ] Delete button removes event from database
- [ ] Deleted event disappears from views

## 10. Settings Dialog
### Opening
- [ ] Opens from File → Settings
- [ ] Dialog centered on screen
- [ ] ESC key closes dialog
- [ ] Current settings displayed

### Appearance
- [ ] Theme dropdown shows current theme
- [ ] Light theme selectable
- [ ] Dark theme selectable

### Calendar Settings
- [ ] First day of week dropdown works
- [ ] All 7 days available (Sun-Sat)
- [ ] Work week start day dropdown works (Mon-Fri)
- [ ] Work week end day dropdown works (Mon-Fri)
- [ ] Warning shows if start > end

### Time Settings
- [ ] Time format dropdown works (12h/24h)
- [ ] Date format dropdown works (3 formats)
- [ ] Time slot interval dropdown works (15/30/45/60)
- [ ] Default start time field accepts HH:MM
- [ ] Invalid time format shows warning

### View Settings
- [ ] Default view dropdown works (all 5 views)
- [ ] "Show My Day panel" checkbox works
- [ ] "Position on right side" checkbox works (indented)
- [ ] "Show ribbon" checkbox works

### Save/Cancel/Reset
- [ ] Cancel closes without saving
- [ ] Save validates settings
- [ ] Save persists to database
- [ ] Theme applies immediately after save
- [ ] Reset button restores defaults
- [ ] Settings persist after app restart

## 11. Theme Picker
### Opening
- [ ] Opens from Theme → Change Theme
- [ ] Dialog centered on screen
- [ ] ESC key closes dialog

### Theme Selection
- [ ] Light theme button works
- [ ] Dark theme button works
- [ ] Selected theme highlighted
- [ ] Preview section updates immediately
- [ ] Preview shows sample text, heading, button, checkbox

### Application
- [ ] Theme applies to entire app immediately
- [ ] All views update with new theme
- [ ] All dialogs update with new theme
- [ ] Theme persists after dialog closed
- [ ] Theme persists after app restart

## 12. Keyboard Shortcuts
- [ ] ESC closes event dialog
- [ ] ESC closes settings dialog
- [ ] ESC closes theme picker
- [ ] ESC closes dialogs in priority order

## 13. Database Persistence
### Events
- [ ] Created events persist after app restart
- [ ] Updated events persist after app restart
- [ ] Deleted events removed after app restart
- [ ] Recurring events expand correctly

### Settings
- [ ] Changed settings persist after app restart
- [ ] Theme persists after app restart
- [ ] Work week settings persist
- [ ] Time slot interval persists

## 14. Recurrence Testing
### Simple Recurrence
- [ ] Daily recurring event shows every day
- [ ] Weekly recurring event shows every week
- [ ] Monthly recurring event shows every month
- [ ] Yearly recurring event shows every year

### Interval
- [ ] Every 2 days works
- [ ] Every 2 weeks works
- [ ] Every 3 months works
- [ ] Custom intervals work

### BYDAY
- [ ] Weekly on Mon/Wed/Fri shows correctly
- [ ] Weekly on weekends only works
- [ ] Monthly on specific days works
- [ ] Multiple BYDAY days work

### End Conditions
- [ ] COUNT: Event stops after N occurrences
- [ ] UNTIL: Event stops after date
- [ ] Never: Event continues indefinitely

## 15. Edge Cases
- [ ] Very long event titles handled
- [ ] Very long descriptions handled
- [ ] Events spanning midnight
- [ ] All-day events display correctly
- [ ] Multiple events same time slot
- [ ] Events with no title (should fail validation)
- [ ] Invalid color codes
- [ ] Invalid time formats
- [ ] Work week with no days (should fail)

## 16. Performance
- [ ] Month view renders quickly with many events
- [ ] Scrolling in day view is smooth
- [ ] Dialog opening is instant
- [ ] View switching is fast
- [ ] No lag when typing in fields

## 17. UI/UX
- [ ] All text is readable
- [ ] Colors are appropriate
- [ ] Spacing is consistent
- [ ] Buttons are properly sized
- [ ] Dropdowns work smoothly
- [ ] Checkboxes are clearly visible
- [ ] Radio buttons are clear
- [ ] Error messages are prominent
- [ ] Success feedback is clear

## Issues Found:
(Document any bugs or issues discovered during testing)

1. 

## Notes:
(Additional observations or suggestions)

1. Event click handlers only in Month view - need to add to Day/Week/WorkWeek views
2. Consider adding event search/filter
3. Could improve date picker (currently manual navigation)
4. Export/Import iCalendar functionality exists but not wired to UI
