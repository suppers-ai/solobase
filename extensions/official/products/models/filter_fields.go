package models

import "fmt"

// FilterFieldID represents the available filter field IDs
type FilterFieldID string

// Filter field ID constants - these map to the database columns
const (
	// Text filter fields
	FilterText1 FilterFieldID = "filter_text_1"
	FilterText2 FilterFieldID = "filter_text_2"
	FilterText3 FilterFieldID = "filter_text_3"
	FilterText4 FilterFieldID = "filter_text_4"
	FilterText5 FilterFieldID = "filter_text_5"

	// Numeric filter fields
	FilterNumeric1 FilterFieldID = "filter_numeric_1"
	FilterNumeric2 FilterFieldID = "filter_numeric_2"
	FilterNumeric3 FilterFieldID = "filter_numeric_3"
	FilterNumeric4 FilterFieldID = "filter_numeric_4"
	FilterNumeric5 FilterFieldID = "filter_numeric_5"

	// Boolean filter fields
	FilterBoolean1 FilterFieldID = "filter_boolean_1"
	FilterBoolean2 FilterFieldID = "filter_boolean_2"
	FilterBoolean3 FilterFieldID = "filter_boolean_3"
	FilterBoolean4 FilterFieldID = "filter_boolean_4"
	FilterBoolean5 FilterFieldID = "filter_boolean_5"

	// Enum filter fields (for select/multiselect)
	FilterEnum1 FilterFieldID = "filter_enum_1"
	FilterEnum2 FilterFieldID = "filter_enum_2"
	FilterEnum3 FilterFieldID = "filter_enum_3"
	FilterEnum4 FilterFieldID = "filter_enum_4"
	FilterEnum5 FilterFieldID = "filter_enum_5"
)

// String returns the string representation of the FilterFieldID
func (f FilterFieldID) String() string {
	return string(f)
}

// IsValid checks if the filter field ID is valid
func (f FilterFieldID) IsValid() bool {
	switch f {
	case FilterText1, FilterText2, FilterText3, FilterText4, FilterText5,
		FilterNumeric1, FilterNumeric2, FilterNumeric3, FilterNumeric4, FilterNumeric5,
		FilterBoolean1, FilterBoolean2, FilterBoolean3, FilterBoolean4, FilterBoolean5,
		FilterEnum1, FilterEnum2, FilterEnum3, FilterEnum4, FilterEnum5:
		return true
	default:
		return false
	}
}

// GetType returns the type of the filter field (text, numeric, boolean, enum)
func (f FilterFieldID) GetType() string {
	switch f {
	case FilterText1, FilterText2, FilterText3, FilterText4, FilterText5:
		return "text"
	case FilterNumeric1, FilterNumeric2, FilterNumeric3, FilterNumeric4, FilterNumeric5:
		return "numeric"
	case FilterBoolean1, FilterBoolean2, FilterBoolean3, FilterBoolean4, FilterBoolean5:
		return "boolean"
	case FilterEnum1, FilterEnum2, FilterEnum3, FilterEnum4, FilterEnum5:
		return "enum"
	default:
		return ""
	}
}

// GetIndex returns the index of the filter field (1-5)
func (f FilterFieldID) GetIndex() int {
	switch f {
	case FilterText1, FilterNumeric1, FilterBoolean1, FilterEnum1:
		return 1
	case FilterText2, FilterNumeric2, FilterBoolean2, FilterEnum2:
		return 2
	case FilterText3, FilterNumeric3, FilterBoolean3, FilterEnum3:
		return 3
	case FilterText4, FilterNumeric4, FilterBoolean4, FilterEnum4:
		return 4
	case FilterText5, FilterNumeric5, FilterBoolean5, FilterEnum5:
		return 5
	default:
		return 0
	}
}

// ValidateFilterFieldID validates and returns a FilterFieldID from a string
func ValidateFilterFieldID(id string) (FilterFieldID, bool) {
	fieldID := FilterFieldID(id)
	if fieldID.IsValid() {
		return fieldID, true
	}
	return "", false
}

// ValidateFieldDefinitions validates a slice of field definitions
func ValidateFieldDefinitions(fields []FieldDefinition) error {
	usedIDs := make(map[FilterFieldID]bool)

	for i, field := range fields {
		// Validate the field itself
		if err := field.Validate(); err != nil {
			return fmt.Errorf("field %d (%s): %w", i, field.Name, err)
		}

		// Check for duplicate IDs
		fieldID := FilterFieldID(field.ID)
		if usedIDs[fieldID] {
			return fmt.Errorf("duplicate field ID: %s", field.ID)
		}
		usedIDs[fieldID] = true
	}

	return nil
}