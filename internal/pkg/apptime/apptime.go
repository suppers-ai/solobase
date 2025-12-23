// Package apptime provides unified time utilities for the application.
// It works consistently across standard Go and TinyGo/WASM builds.
//
// All times are stored and transmitted using RFC3339 format in UTC timezone,
// which is compatible with SQLite (TEXT), PostgreSQL (TIMESTAMP), and JSON APIs.
//
// IMPORTANT: All time functions return UTC time for consistency. Database queries
// should pass apptime.NowTime() as a parameter rather than using database-specific
// functions like datetime('now') or NOW() for portability across SQLite and PostgreSQL.
//
// Time is an alias for time.Time for compatibility with external libraries.
// Use the helper functions for consistent formatting and parsing.
package apptime

import (
	"database/sql"
	"database/sql/driver"
	"fmt"
	"time"
)

// Time is an alias for time.Time for compatibility with external libraries
type Time = time.Time

// TimeFormat is the standard format used for storing and transmitting times.
// RFC3339 is compatible with SQLite, PostgreSQL, and JSON APIs.
const TimeFormat = time.RFC3339

// TimeFormatNano is RFC3339 with nanosecond precision
const TimeFormatNano = time.RFC3339Nano

// Common layout constants (re-exported from time package)
const (
	Layout      = time.Layout
	ANSIC       = time.ANSIC
	UnixDate    = time.UnixDate
	RubyDate    = time.RubyDate
	RFC822      = time.RFC822
	RFC822Z     = time.RFC822Z
	RFC850      = time.RFC850
	RFC1123     = time.RFC1123
	RFC1123Z    = time.RFC1123Z
	RFC3339     = time.RFC3339
	RFC3339Nano = time.RFC3339Nano
	Kitchen     = time.Kitchen
	Stamp       = time.Stamp
	StampMilli  = time.StampMilli
	StampMicro  = time.StampMicro
	StampNano   = time.StampNano
	DateTime    = time.DateTime
	DateOnly    = time.DateOnly
	TimeOnly    = time.TimeOnly
)

// Now returns the current UTC time formatted as a string
func Now() string {
	return time.Now().UTC().Format(TimeFormat)
}

// NowTime returns the current UTC time
func NowTime() Time {
	return time.Now().UTC()
}

// NowString returns the current UTC time as a formatted string
func NowString() string {
	return time.Now().UTC().Format(TimeFormat)
}

// Format converts a time.Time to a formatted string
func Format(t Time) string {
	if t.IsZero() {
		return ""
	}
	return t.Format(TimeFormat)
}

// FormatPtr formats a time pointer, returning nil for nil input
func FormatPtr(t *Time) *string {
	if t == nil {
		return nil
	}
	s := Format(*t)
	return &s
}

// Parse parses a formatted time string using the default TimeFormat (RFC3339)
func Parse(s string) (Time, error) {
	if s == "" {
		return time.Time{}, nil
	}
	return time.Parse(TimeFormat, s)
}

// ParseWithLayout parses a time string using a custom layout
func ParseWithLayout(layout, value string) (Time, error) {
	return time.Parse(layout, value)
}

// MustParse parses a time string or returns zero time on error
func MustParse(s string) Time {
	t, _ := Parse(s)
	return t
}

// MustParseWithLayout parses a time string with custom layout or returns zero time on error
func MustParseWithLayout(layout, value string) Time {
	t, _ := ParseWithLayout(layout, value)
	return t
}

// NewTime creates a new Time (for API compatibility, just returns the input)
func NewTime(t Time) Time {
	return t
}

// NullTime represents a nullable time with consistent serialization
type NullTime struct {
	Time  time.Time
	Valid bool
}

// Scan implements sql.Scanner
// All scanned times are converted to UTC for consistency
func (nt *NullTime) Scan(value interface{}) error {
	if value == nil {
		nt.Time = time.Time{}
		nt.Valid = false
		return nil
	}

	switch v := value.(type) {
	case time.Time:
		nt.Time = v.UTC()
		nt.Valid = true
		return nil
	case string:
		parsed, err := Parse(v)
		if err != nil {
			// Try other common formats
			for _, format := range []string{
				"2006-01-02 15:04:05",
				"2006-01-02T15:04:05Z",
				"2006-01-02T15:04:05",
				"2006-01-02",
			} {
				if parsed, err = time.Parse(format, v); err == nil {
					nt.Time = parsed.UTC()
					nt.Valid = true
					return nil
				}
			}
			return fmt.Errorf("cannot parse time: %s", v)
		}
		nt.Time = parsed.UTC()
		nt.Valid = true
		return nil
	case []byte:
		return nt.Scan(string(v))
	default:
		return fmt.Errorf("cannot scan type %T into NullTime", value)
	}
}

// Value implements driver.Valuer
// Times are always stored in UTC format
func (nt NullTime) Value() (driver.Value, error) {
	if !nt.Valid {
		return nil, nil
	}
	return nt.Time.UTC().Format(TimeFormat), nil
}

// ToTimePtr returns a *time.Time if Valid, nil otherwise
func (nt NullTime) ToTimePtr() *Time {
	if !nt.Valid {
		return nil
	}
	return &nt.Time
}

// FromTimePtr creates a NullTime from a *time.Time
func FromTimePtr(t *Time) NullTime {
	if t == nil {
		return NullTime{Valid: false}
	}
	return NullTime{Time: *t, Valid: true}
}

// ToSQLValue returns a value compatible with SQL drivers.
// Returns string for valid times in UTC, nil for invalid (NULL in database)
func (nt NullTime) ToSQLValue() interface{} {
	if !nt.Valid {
		return nil
	}
	return nt.Time.UTC().Format(TimeFormat)
}

// NewNullTime creates a NullTime from a time.Time, converting to UTC
func NewNullTime(t Time) NullTime {
	return NullTime{Time: t.UTC(), Valid: true}
}

// NewNullTimeNow returns the current UTC time as a NullTime
func NewNullTimeNow() NullTime {
	return NullTime{Time: time.Now().UTC(), Valid: true}
}

// NewNullTimeExpiry creates a NullTime that expires after the given duration from now (UTC)
func NewNullTimeExpiry(d Duration) NullTime {
	return NullTime{Time: time.Now().UTC().Add(d), Valid: true}
}

// Duration is an alias for time.Duration
type Duration = time.Duration

// Duration constants
const (
	Nanosecond  Duration = time.Nanosecond
	Microsecond Duration = time.Microsecond
	Millisecond Duration = time.Millisecond
	Second      Duration = time.Second
	Minute      Duration = time.Minute
	Hour        Duration = time.Hour
)

// Sleep pauses the current goroutine for at least the duration d
func Sleep(d Duration) {
	time.Sleep(d)
}

// After waits for the duration to elapse and then sends the current time on the returned channel
func After(d Duration) <-chan Time {
	return time.After(d)
}

// Since returns the time elapsed since t
func Since(t Time) Duration {
	return time.Since(t)
}

// Until returns the duration until t
func Until(t Time) Duration {
	return time.Until(t)
}

// Ticker wraps time.Ticker
type Ticker = time.Ticker

// NewTicker returns a new Ticker containing a channel that will send the current time
func NewTicker(d Duration) *Ticker {
	return time.NewTicker(d)
}

// Timer wraps time.Timer
type Timer = time.Timer

// NewTimer creates a new Timer that will send the current time on its channel after at least duration d
func NewTimer(d Duration) *Timer {
	return time.NewTimer(d)
}

// AfterFunc waits for the duration to elapse and then calls f in its own goroutine
func AfterFunc(d Duration, f func()) *Timer {
	return time.AfterFunc(d, f)
}

// ParseDuration parses a duration string
func ParseDuration(s string) (Duration, error) {
	return time.ParseDuration(s)
}

// Date returns the Time corresponding to the given date
func Date(year int, month Month, day, hour, min, sec, nsec int, loc *Location) Time {
	if loc == nil {
		loc = time.UTC
	}
	return time.Date(year, month, day, hour, min, sec, nsec, loc)
}

// Unix returns the UTC Time corresponding to the given Unix time
func Unix(sec int64, nsec int64) Time {
	return time.Unix(sec, nsec).UTC()
}

// Month type alias
type Month = time.Month

// Month constants
const (
	January   Month = time.January
	February  Month = time.February
	March     Month = time.March
	April     Month = time.April
	May       Month = time.May
	June      Month = time.June
	July      Month = time.July
	August    Month = time.August
	September Month = time.September
	October   Month = time.October
	November  Month = time.November
	December  Month = time.December
)

// Weekday type alias
type Weekday = time.Weekday

// Weekday constants
const (
	Sunday    Weekday = time.Sunday
	Monday    Weekday = time.Monday
	Tuesday   Weekday = time.Tuesday
	Wednesday Weekday = time.Wednesday
	Thursday  Weekday = time.Thursday
	Friday    Weekday = time.Friday
	Saturday  Weekday = time.Saturday
)

// Location wraps time.Location
type Location = time.Location

// UTC is the UTC timezone
var UTC = time.UTC

// Local is the local timezone
var Local = time.Local

// LoadLocation returns the Location with the given name
func LoadLocation(name string) (*Location, error) {
	return time.LoadLocation(name)
}

// FixedZone returns a Location that always uses the given zone name and offset
func FixedZone(name string, offset int) *Location {
	return time.FixedZone(name, offset)
}

// NullTimeScanner is a helper for scanning nullable times from database
// Usage: var nt apptime.NullTime; db.QueryRow(...).Scan(apptime.NullTimeScanner(&nt))
func NullTimeScanner(nt *NullTime) sql.Scanner {
	return nt
}

// FormatNullTime formats a NullTime as string, returns empty string if not valid
func FormatNullTime(nt NullTime) string {
	if !nt.Valid {
		return ""
	}
	return Format(nt.Time)
}
