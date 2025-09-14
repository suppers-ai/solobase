package dynamicfields

import (
	"encoding/json"
	"fmt"
	"strconv"
	"time"
)

// Mapper handles field value transformations and mappings
type Mapper struct {
	schema *Schema
}

// NewMapper creates a new mapper for a schema
func NewMapper(schema *Schema) *Mapper {
	return &Mapper{schema: schema}
}

// MapToDocument maps raw data to a document structure
func (m *Mapper) MapToDocument(id string, data map[string]interface{}) (*Document, error) {
	doc := &Document{
		ID:        id,
		SchemaID:  m.schema.Name,
		Values:    make(map[string]interface{}),
		Metadata:  make(map[string]interface{}),
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}

	for _, field := range m.schema.Fields {
		if value, exists := data[field.Name]; exists {
			mapped, err := m.MapValue(field, value)
			if err != nil {
				return nil, fmt.Errorf("field '%s': %w", field.Name, err)
			}
			doc.Values[field.Name] = mapped
		} else if field.DefaultValue != nil {
			doc.Values[field.Name] = field.DefaultValue
		}
	}

	return doc, nil
}

// MapValue transforms a raw value to the appropriate field type
func (m *Mapper) MapValue(field *Field, value interface{}) (interface{}, error) {
	if value == nil {
		return nil, nil
	}

	switch field.Type {
	case FieldTypeText, FieldTypeEmail, FieldTypeURL:
		return m.mapToString(value)

	case FieldTypeNumber:
		return m.mapToNumber(value)

	case FieldTypeBoolean:
		return m.mapToBoolean(value)

	case FieldTypeDate:
		return m.mapToDate(value)

	case FieldTypeDateTime:
		return m.mapToDateTime(value)

	case FieldTypeEnum:
		// Validate against options
		for _, opt := range field.Options {
			if opt.Value == value {
				return value, nil
			}
		}
		return nil, fmt.Errorf("value '%v' is not a valid option", value)

	case FieldTypeArray:
		return m.mapToArray(field, value)

	case FieldTypeObject:
		return m.mapToObject(field, value)

	case FieldTypeGeo:
		return m.mapToGeoPoint(value)

	case FieldTypeFile, FieldTypeImage:
		// These typically store metadata about the file
		return m.mapToFileMetadata(value)

	default:
		return value, nil
	}
}

// mapToString converts various types to string
func (m *Mapper) mapToString(value interface{}) (string, error) {
	switch v := value.(type) {
	case string:
		return v, nil
	case []byte:
		return string(v), nil
	case fmt.Stringer:
		return v.String(), nil
	default:
		return fmt.Sprintf("%v", value), nil
	}
}

// mapToNumber converts various types to float64
func (m *Mapper) mapToNumber(value interface{}) (float64, error) {
	switch v := value.(type) {
	case float64:
		return v, nil
	case float32:
		return float64(v), nil
	case int:
		return float64(v), nil
	case int32:
		return float64(v), nil
	case int64:
		return float64(v), nil
	case string:
		return strconv.ParseFloat(v, 64)
	default:
		return 0, fmt.Errorf("cannot convert %T to number", value)
	}
}

// mapToBoolean converts various types to boolean
func (m *Mapper) mapToBoolean(value interface{}) (bool, error) {
	switch v := value.(type) {
	case bool:
		return v, nil
	case string:
		return strconv.ParseBool(v)
	case int, int32, int64:
		return v != 0, nil
	case float32, float64:
		return v != 0, nil
	default:
		return false, fmt.Errorf("cannot convert %T to boolean", value)
	}
}

// mapToDate converts various types to date
func (m *Mapper) mapToDate(value interface{}) (time.Time, error) {
	switch v := value.(type) {
	case time.Time:
		// Truncate to date only
		return time.Date(v.Year(), v.Month(), v.Day(), 0, 0, 0, 0, v.Location()), nil
	case string:
		// Try various date formats
		formats := []string{
			"2006-01-02",
			"2006/01/02",
			"01/02/2006",
			"02/01/2006",
			time.RFC3339,
		}
		for _, format := range formats {
			if t, err := time.Parse(format, v); err == nil {
				return time.Date(t.Year(), t.Month(), t.Day(), 0, 0, 0, 0, t.Location()), nil
			}
		}
		return time.Time{}, fmt.Errorf("cannot parse date: %s", v)
	default:
		return time.Time{}, fmt.Errorf("cannot convert %T to date", value)
	}
}

// mapToDateTime converts various types to datetime
func (m *Mapper) mapToDateTime(value interface{}) (time.Time, error) {
	switch v := value.(type) {
	case time.Time:
		return v, nil
	case string:
		// Try various datetime formats
		formats := []string{
			time.RFC3339,
			time.RFC3339Nano,
			"2006-01-02 15:04:05",
			"2006-01-02T15:04:05",
			"2006/01/02 15:04:05",
		}
		for _, format := range formats {
			if t, err := time.Parse(format, v); err == nil {
				return t, nil
			}
		}
		return time.Time{}, fmt.Errorf("cannot parse datetime: %s", v)
	case int64:
		// Assume Unix timestamp
		return time.Unix(v, 0), nil
	default:
		return time.Time{}, fmt.Errorf("cannot convert %T to datetime", value)
	}
}

// mapToArray converts various types to array
func (m *Mapper) mapToArray(field *Field, value interface{}) ([]interface{}, error) {
	switch v := value.(type) {
	case []interface{}:
		if field.Items != nil {
			// Map each item
			result := make([]interface{}, len(v))
			for i, item := range v {
				mapped, err := m.MapValue(field.Items, item)
				if err != nil {
					return nil, fmt.Errorf("item %d: %w", i, err)
				}
				result[i] = mapped
			}
			return result, nil
		}
		return v, nil
	case string:
		// Try to parse as JSON array
		var arr []interface{}
		if err := json.Unmarshal([]byte(v), &arr); err == nil {
			return m.mapToArray(field, arr)
		}
		// Single value to array
		return []interface{}{v}, nil
	default:
		// Convert single value to array
		return []interface{}{value}, nil
	}
}

// mapToObject converts various types to object
func (m *Mapper) mapToObject(field *Field, value interface{}) (map[string]interface{}, error) {
	switch v := value.(type) {
	case map[string]interface{}:
		if field.Properties != nil {
			// Map each property
			result := make(map[string]interface{})
			for propName, propField := range field.Properties {
				if propValue, exists := v[propName]; exists {
					mapped, err := m.MapValue(propField, propValue)
					if err != nil {
						return nil, fmt.Errorf("property '%s': %w", propName, err)
					}
					result[propName] = mapped
				} else if propField.DefaultValue != nil {
					result[propName] = propField.DefaultValue
				}
			}
			return result, nil
		}
		return v, nil
	case string:
		// Try to parse as JSON object
		var obj map[string]interface{}
		if err := json.Unmarshal([]byte(v), &obj); err == nil {
			return m.mapToObject(field, obj)
		}
		return nil, fmt.Errorf("cannot parse object from string")
	default:
		return nil, fmt.Errorf("cannot convert %T to object", value)
	}
}

// mapToGeoPoint converts various types to GeoPoint
func (m *Mapper) mapToGeoPoint(value interface{}) (GeoPoint, error) {
	switch v := value.(type) {
	case GeoPoint:
		return v, nil
	case map[string]interface{}:
		lat, latOk := v["lat"]
		lng, lngOk := v["lng"]
		if !latOk || !lngOk {
			return GeoPoint{}, fmt.Errorf("geo point must have lat and lng fields")
		}

		latFloat, err := m.mapToNumber(lat)
		if err != nil {
			return GeoPoint{}, fmt.Errorf("invalid latitude: %w", err)
		}

		lngFloat, err := m.mapToNumber(lng)
		if err != nil {
			return GeoPoint{}, fmt.Errorf("invalid longitude: %w", err)
		}

		return GeoPoint{Lat: latFloat, Lng: lngFloat}, nil
	case string:
		// Try to parse as JSON
		var point GeoPoint
		if err := json.Unmarshal([]byte(v), &point); err == nil {
			return point, nil
		}
		return GeoPoint{}, fmt.Errorf("cannot parse geo point from string")
	default:
		return GeoPoint{}, fmt.Errorf("cannot convert %T to geo point", value)
	}
}

// mapToFileMetadata converts various types to file metadata
func (m *Mapper) mapToFileMetadata(value interface{}) (map[string]interface{}, error) {
	switch v := value.(type) {
	case map[string]interface{}:
		return v, nil
	case string:
		// Assume it's a file path or URL
		return map[string]interface{}{
			"url": v,
		}, nil
	default:
		return nil, fmt.Errorf("cannot convert %T to file metadata", value)
	}
}

// ExtractValues extracts field values from a document
func (m *Mapper) ExtractValues(doc *Document) map[string]interface{} {
	result := make(map[string]interface{})

	for _, field := range m.schema.Fields {
		if value, exists := doc.Values[field.Name]; exists {
			result[field.Name] = m.extractValue(field, value)
		}
	}

	return result
}

// extractValue converts internal representation to external format
func (m *Mapper) extractValue(field *Field, value interface{}) interface{} {
	if value == nil {
		return nil
	}

	switch field.Type {
	case FieldTypeDate:
		if t, ok := value.(time.Time); ok {
			return t.Format("2006-01-02")
		}
	case FieldTypeDateTime:
		if t, ok := value.(time.Time); ok {
			return t.Format(time.RFC3339)
		}
	case FieldTypeGeo:
		if point, ok := value.(GeoPoint); ok {
			return map[string]float64{
				"lat": point.Lat,
				"lng": point.Lng,
			}
		}
	}

	return value
}

// ApplyDefaults applies default values to missing fields
func (m *Mapper) ApplyDefaults(data map[string]interface{}) map[string]interface{} {
	result := make(map[string]interface{})

	// Copy existing values
	for k, v := range data {
		result[k] = v
	}

	// Apply defaults for missing fields
	for _, field := range m.schema.Fields {
		if _, exists := result[field.Name]; !exists && field.DefaultValue != nil {
			result[field.Name] = field.DefaultValue
		}
	}

	return result
}

// FilterFields removes fields not defined in the schema
func (m *Mapper) FilterFields(data map[string]interface{}) map[string]interface{} {
	result := make(map[string]interface{})

	for _, field := range m.schema.Fields {
		if value, exists := data[field.Name]; exists {
			result[field.Name] = value
		}
	}

	return result
}
