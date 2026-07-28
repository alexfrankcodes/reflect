# Reflect

A tray-only desktop journal that interrupts you once a day, at a time you chose, and offers a blank page.
Everything it keeps is a plain text file on your own machine.

## Language

### The journal

**Entry**:
What the user wrote on one day, stored as that day's file.
A day nobody wrote on has no entry and no file — absence is how an unwritten day is represented, never an empty file.
_Avoid_: note, post, journal entry, record

**Day**:
A calendar date, and the unit the journal is divided into.
There is at most one entry per day.
_Avoid_: date (when the day is meant), session

**Entries folder**:
The single folder holding every entry, in the OS's per-app data location.
It is the whole of the user's data — there is no database and no index, because the folder is the index.
_Avoid_: store, vault, library, archive

**Notes session**:
One sitting at the notes window: the day it is open on, the prompt it was opened with, and what was on the page when it opened.
It is what decides whether closing the window is worth writing to disk.
_Avoid_: draft, editing session, buffer

### The nudge

**Reminder**:
The daily notification, and the only thing that interrupts the user unbidden.
"Nudge" is deliberate prose in the settings window's confirmation line; everywhere else this is a reminder.
_Avoid_: notification, alert, ping, notice

**Daily time**:
The single time of day, chosen by the user, that the reminder is due at.
One time a day, every day — not a schedule, not per-weekday.
_Avoid_: reflection time, reminder time, schedule, alarm

**Occurrence**:
A particular day's instance of the daily time — the scheduled moment a reminder belongs to, as distinct from the moment it actually appears.
A catch-up shown on Tuesday morning is still Monday evening's occurrence, and that is what gets recorded as delivered.
_Avoid_: trigger, event, firing, instance

**Catch-up**:
A reminder shown late because the machine was asleep or off at its occurrence.
Let go entirely when the machine wakes close enough to the next occurrence that two prompts would arrive nearly back to back.
_Avoid_: missed reminder, late reminder, backfill, retry

**Skip**:
A day that passes with no entry, whether the reminder was dismissed or the notes window was closed with nothing written.
A skip is silent and complete: Reflect does not ask again that day.
_Avoid_: miss, ignore, snooze, dismiss

### The page

**Prompt**:
The short open question offered above the blank page as somewhere to start.
Chosen from the date, so a day always shows the same one, and it can be turned off entirely.
_Avoid_: question, seed, suggestion, placeholder

**Prompt library**:
The fixed set of prompts Reflect ships with.
Every prompt in it appears before any repeats.
_Avoid_: prompt list, prompt pool, prompt bank
