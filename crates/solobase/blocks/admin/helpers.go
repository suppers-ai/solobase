package admin

// strVal extracts a string from any value.
func strVal(v any) string {
	if s, ok := v.(string); ok {
		return s
	}
	return ""
}

// boolVal extracts a bool from any value.
func boolVal(v any) bool {
	switch b := v.(type) {
	case bool:
		return b
	case int64:
		return b != 0
	case float64:
		return b != 0
	}
	return false
}

// toInt64 converts various numeric types to int64.
func toInt64(v any) int64 {
	switch val := v.(type) {
	case int64:
		return val
	case float64:
		return int64(val)
	case int:
		return int64(val)
	default:
		return 0
	}
}
