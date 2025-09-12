package dynamicfields

import (
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestMapper_MapToDocument(t *testing.T) {
	schema := &Schema{
		Name: "test",
		Fields: []*Field{
			{
				Name: "name",
				Type: FieldTypeText,
			},
			{
				Name:         "age",
				Type:         FieldTypeNumber,
				DefaultValue: 18.0,
			},
			{
				Name: "active",
				Type: FieldTypeBoolean,
			},
		},
	}

	mapper := NewMapper(schema)

	tests := []struct {
		name     string
		id       string
		data     map[string]interface{}
		expected map[string]interface{}
	}{
		{
			name: "all values provided",
			id:   "doc1",
			data: map[string]interface{}{
				"name":   "John Doe",
				"age":    30,
				"active": true,
			},
			expected: map[string]interface{}{
				"name":   "John Doe",
				"age":    30.0,
				"active": true,
			},
		},
		{
			name: "with default value",
			id:   "doc2",
			data: map[string]interface{}{
				"name":   "Jane Doe",
				"active": false,
			},
			expected: map[string]interface{}{
				"name":   "Jane Doe",
				"age":    18.0,
				"active": false,
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			doc, err := mapper.MapToDocument(tt.id, tt.data)
			require.NoError(t, err)
			assert.Equal(t, tt.id, doc.ID)
			assert.Equal(t, schema.Name, doc.SchemaID)
			assert.Equal(t, tt.expected, doc.Values)
		})
	}
}

func TestMapper_MapValue(t *testing.T) {
	mapper := NewMapper(&Schema{})

	tests := []struct {
		name     string
		field    *Field
		value    interface{}
		expected interface{}
		wantErr  bool
	}{
		// String conversions
		{
			name:     "string to text",
			field:    &Field{Type: FieldTypeText},
			value:    "hello",
			expected: "hello",
		},
		{
			name:     "number to text",
			field:    &Field{Type: FieldTypeText},
			value:    123,
			expected: "123",
		},
		// Number conversions
		{
			name:     "int to number",
			field:    &Field{Type: FieldTypeNumber},
			value:    42,
			expected: 42.0,
		},
		{
			name:     "string to number",
			field:    &Field{Type: FieldTypeNumber},
			value:    "3.14",
			expected: 3.14,
		},
		{
			name:    "invalid string to number",
			field:   &Field{Type: FieldTypeNumber},
			value:   "not a number",
			wantErr: true,
		},
		// Boolean conversions
		{
			name:     "bool to boolean",
			field:    &Field{Type: FieldTypeBoolean},
			value:    true,
			expected: true,
		},
		{
			name:     "string to boolean",
			field:    &Field{Type: FieldTypeBoolean},
			value:    "true",
			expected: true,
		},
		{
			name:     "number to boolean",
			field:    &Field{Type: FieldTypeBoolean},
			value:    1,
			expected: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := mapper.MapValue(tt.field, tt.value)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				require.NoError(t, err)
				assert.Equal(t, tt.expected, result)
			}
		})
	}
}

func TestMapper_MapToDate(t *testing.T) {
	mapper := NewMapper(&Schema{})
	field := &Field{Type: FieldTypeDate}

	tests := []struct {
		name     string
		value    interface{}
		expected string
		wantErr  bool
	}{
		{
			name:     "time.Time to date",
			value:    time.Date(2023, 6, 15, 10, 30, 0, 0, time.UTC),
			expected: "2023-06-15",
		},
		{
			name:     "ISO date string",
			value:    "2023-06-15",
			expected: "2023-06-15",
		},
		{
			name:     "slash date format",
			value:    "2023/06/15",
			expected: "2023-06-15",
		},
		{
			name:    "invalid date string",
			value:   "not a date",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := mapper.MapValue(field, tt.value)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				require.NoError(t, err)
				if resultTime, ok := result.(time.Time); ok {
					assert.Equal(t, tt.expected, resultTime.Format("2006-01-02"))
				}
			}
		})
	}
}

func TestMapper_MapToArray(t *testing.T) {
	mapper := NewMapper(&Schema{})

	tests := []struct {
		name     string
		field    *Field
		value    interface{}
		expected []interface{}
		wantErr  bool
	}{
		{
			name: "array of strings",
			field: &Field{
				Type:  FieldTypeArray,
				Items: &Field{Type: FieldTypeText},
			},
			value:    []interface{}{"a", "b", "c"},
			expected: []interface{}{"a", "b", "c"},
		},
		{
			name: "array with type conversion",
			field: &Field{
				Type:  FieldTypeArray,
				Items: &Field{Type: FieldTypeNumber},
			},
			value:    []interface{}{1, "2", 3.0},
			expected: []interface{}{1.0, 2.0, 3.0},
		},
		{
			name: "single value to array",
			field: &Field{
				Type: FieldTypeArray,
			},
			value:    "single",
			expected: []interface{}{"single"},
		},
		{
			name: "JSON string to array",
			field: &Field{
				Type: FieldTypeArray,
			},
			value:    `["a", "b", "c"]`,
			expected: []interface{}{"a", "b", "c"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := mapper.MapValue(tt.field, tt.value)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				require.NoError(t, err)
				assert.Equal(t, tt.expected, result)
			}
		})
	}
}

func TestMapper_MapToObject(t *testing.T) {
	mapper := NewMapper(&Schema{})

	field := &Field{
		Type: FieldTypeObject,
		Properties: map[string]*Field{
			"name": {
				Name: "name",
				Type: FieldTypeText,
			},
			"age": {
				Name:         "age",
				Type:         FieldTypeNumber,
				DefaultValue: 0.0,
			},
		},
	}

	tests := []struct {
		name     string
		value    interface{}
		expected map[string]interface{}
		wantErr  bool
	}{
		{
			name: "valid object",
			value: map[string]interface{}{
				"name": "John",
				"age":  30,
			},
			expected: map[string]interface{}{
				"name": "John",
				"age":  30.0,
			},
		},
		{
			name: "with default value",
			value: map[string]interface{}{
				"name": "Jane",
			},
			expected: map[string]interface{}{
				"name": "Jane",
				"age":  0.0,
			},
		},
		{
			name:  "JSON string to object",
			value: `{"name": "Bob", "age": 25}`,
			expected: map[string]interface{}{
				"name": "Bob",
				"age":  25.0,
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := mapper.MapValue(field, tt.value)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				require.NoError(t, err)
				assert.Equal(t, tt.expected, result)
			}
		})
	}
}

func TestMapper_MapToGeoPoint(t *testing.T) {
	mapper := NewMapper(&Schema{})
	field := &Field{Type: FieldTypeGeo}

	tests := []struct {
		name     string
		value    interface{}
		expected GeoPoint
		wantErr  bool
	}{
		{
			name:     "GeoPoint struct",
			value:    GeoPoint{Lat: 40.7128, Lng: -74.0060},
			expected: GeoPoint{Lat: 40.7128, Lng: -74.0060},
		},
		{
			name: "map with lat/lng",
			value: map[string]interface{}{
				"lat": 40.7128,
				"lng": -74.0060,
			},
			expected: GeoPoint{Lat: 40.7128, Lng: -74.0060},
		},
		{
			name:     "JSON string",
			value:    `{"lat": 40.7128, "lng": -74.0060}`,
			expected: GeoPoint{Lat: 40.7128, Lng: -74.0060},
		},
		{
			name: "missing lat",
			value: map[string]interface{}{
				"lng": -74.0060,
			},
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result, err := mapper.MapValue(field, tt.value)
			if tt.wantErr {
				assert.Error(t, err)
			} else {
				require.NoError(t, err)
				assert.Equal(t, tt.expected, result)
			}
		})
	}
}

func TestMapper_ApplyDefaults(t *testing.T) {
	schema := &Schema{
		Name: "test",
		Fields: []*Field{
			{
				Name: "name",
				Type: FieldTypeText,
			},
			{
				Name:         "status",
				Type:         FieldTypeText,
				DefaultValue: "pending",
			},
			{
				Name:         "priority",
				Type:         FieldTypeNumber,
				DefaultValue: 1.0,
			},
		},
	}

	mapper := NewMapper(schema)

	data := map[string]interface{}{
		"name": "Task 1",
	}

	result := mapper.ApplyDefaults(data)

	assert.Equal(t, "Task 1", result["name"])
	assert.Equal(t, "pending", result["status"])
	assert.Equal(t, 1.0, result["priority"])
}

func TestMapper_FilterFields(t *testing.T) {
	schema := &Schema{
		Name: "test",
		Fields: []*Field{
			{Name: "name", Type: FieldTypeText},
			{Name: "age", Type: FieldTypeNumber},
		},
	}

	mapper := NewMapper(schema)

	data := map[string]interface{}{
		"name":    "John",
		"age":     30,
		"unknown": "should be removed",
		"extra":   123,
	}

	result := mapper.FilterFields(data)

	assert.Equal(t, 2, len(result))
	assert.Equal(t, "John", result["name"])
	assert.Equal(t, 30, result["age"])
	assert.Nil(t, result["unknown"])
	assert.Nil(t, result["extra"])
}

func TestMapper_ExtractValues(t *testing.T) {
	schema := &Schema{
		Name: "test",
		Fields: []*Field{
			{Name: "name", Type: FieldTypeText},
			{Name: "birthdate", Type: FieldTypeDate},
			{Name: "location", Type: FieldTypeGeo},
		},
	}

	mapper := NewMapper(schema)

	doc := &Document{
		Values: map[string]interface{}{
			"name":      "John",
			"birthdate": time.Date(1990, 6, 15, 0, 0, 0, 0, time.UTC),
			"location":  GeoPoint{Lat: 40.7128, Lng: -74.0060},
		},
	}

	result := mapper.ExtractValues(doc)

	assert.Equal(t, "John", result["name"])
	assert.Equal(t, "1990-06-15", result["birthdate"])

	location := result["location"].(map[string]float64)
	assert.Equal(t, 40.7128, location["lat"])
	assert.Equal(t, -74.0060, location["lng"])
}
