package middleware

// RecordDBQuery is a no-op (metrics collected via request logging)
func RecordDBQuery(operation string, duration float64, isError bool) {}
