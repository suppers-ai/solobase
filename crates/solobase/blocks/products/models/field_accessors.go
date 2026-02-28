package models

// Field accessor functions for Product struct
// These replace reflection-based field access for TinyGo/WASI compatibility

// GetProductFilterField returns the value of a filter field by its struct field name
func GetProductFilterField(product *Product, fieldName string) interface{} {
	switch fieldName {
	// Numeric fields
	case "FilterNumeric1":
		return product.FilterNumeric1
	case "FilterNumeric2":
		return product.FilterNumeric2
	case "FilterNumeric3":
		return product.FilterNumeric3
	case "FilterNumeric4":
		return product.FilterNumeric4
	case "FilterNumeric5":
		return product.FilterNumeric5

	// Text fields
	case "FilterText1":
		return product.FilterText1
	case "FilterText2":
		return product.FilterText2
	case "FilterText3":
		return product.FilterText3
	case "FilterText4":
		return product.FilterText4
	case "FilterText5":
		return product.FilterText5

	// Boolean fields
	case "FilterBoolean1":
		return product.FilterBoolean1
	case "FilterBoolean2":
		return product.FilterBoolean2
	case "FilterBoolean3":
		return product.FilterBoolean3
	case "FilterBoolean4":
		return product.FilterBoolean4
	case "FilterBoolean5":
		return product.FilterBoolean5

	// Enum fields
	case "FilterEnum1":
		return product.FilterEnum1
	case "FilterEnum2":
		return product.FilterEnum2
	case "FilterEnum3":
		return product.FilterEnum3
	case "FilterEnum4":
		return product.FilterEnum4
	case "FilterEnum5":
		return product.FilterEnum5

	// Location fields
	case "FilterLocation1":
		return product.FilterLocation1
	case "FilterLocation2":
		return product.FilterLocation2
	case "FilterLocation3":
		return product.FilterLocation3
	case "FilterLocation4":
		return product.FilterLocation4
	case "FilterLocation5":
		return product.FilterLocation5

	default:
		return nil
	}
}

// SetProductFilterField sets the value of a filter field by its struct field name
func SetProductFilterField(product *Product, fieldName string, value interface{}) bool {
	switch fieldName {
	// Numeric fields
	case "FilterNumeric1":
		if v, ok := value.(*float64); ok {
			product.FilterNumeric1 = v
			return true
		}
	case "FilterNumeric2":
		if v, ok := value.(*float64); ok {
			product.FilterNumeric2 = v
			return true
		}
	case "FilterNumeric3":
		if v, ok := value.(*float64); ok {
			product.FilterNumeric3 = v
			return true
		}
	case "FilterNumeric4":
		if v, ok := value.(*float64); ok {
			product.FilterNumeric4 = v
			return true
		}
	case "FilterNumeric5":
		if v, ok := value.(*float64); ok {
			product.FilterNumeric5 = v
			return true
		}

	// Text fields
	case "FilterText1":
		if v, ok := value.(*string); ok {
			product.FilterText1 = v
			return true
		}
	case "FilterText2":
		if v, ok := value.(*string); ok {
			product.FilterText2 = v
			return true
		}
	case "FilterText3":
		if v, ok := value.(*string); ok {
			product.FilterText3 = v
			return true
		}
	case "FilterText4":
		if v, ok := value.(*string); ok {
			product.FilterText4 = v
			return true
		}
	case "FilterText5":
		if v, ok := value.(*string); ok {
			product.FilterText5 = v
			return true
		}

	// Boolean fields
	case "FilterBoolean1":
		if v, ok := value.(*bool); ok {
			product.FilterBoolean1 = v
			return true
		}
	case "FilterBoolean2":
		if v, ok := value.(*bool); ok {
			product.FilterBoolean2 = v
			return true
		}
	case "FilterBoolean3":
		if v, ok := value.(*bool); ok {
			product.FilterBoolean3 = v
			return true
		}
	case "FilterBoolean4":
		if v, ok := value.(*bool); ok {
			product.FilterBoolean4 = v
			return true
		}
	case "FilterBoolean5":
		if v, ok := value.(*bool); ok {
			product.FilterBoolean5 = v
			return true
		}

	// Enum fields
	case "FilterEnum1":
		if v, ok := value.(*string); ok {
			product.FilterEnum1 = v
			return true
		}
	case "FilterEnum2":
		if v, ok := value.(*string); ok {
			product.FilterEnum2 = v
			return true
		}
	case "FilterEnum3":
		if v, ok := value.(*string); ok {
			product.FilterEnum3 = v
			return true
		}
	case "FilterEnum4":
		if v, ok := value.(*string); ok {
			product.FilterEnum4 = v
			return true
		}
	case "FilterEnum5":
		if v, ok := value.(*string); ok {
			product.FilterEnum5 = v
			return true
		}

	// Location fields
	case "FilterLocation1":
		if v, ok := value.(*string); ok {
			product.FilterLocation1 = v
			return true
		}
	case "FilterLocation2":
		if v, ok := value.(*string); ok {
			product.FilterLocation2 = v
			return true
		}
	case "FilterLocation3":
		if v, ok := value.(*string); ok {
			product.FilterLocation3 = v
			return true
		}
	case "FilterLocation4":
		if v, ok := value.(*string); ok {
			product.FilterLocation4 = v
			return true
		}
	case "FilterLocation5":
		if v, ok := value.(*string); ok {
			product.FilterLocation5 = v
			return true
		}
	}
	return false
}

// CopyProductFilterField copies a filter field value from one product to another
func CopyProductFilterField(dst *Product, src *Product, fieldName string) {
	switch fieldName {
	// Numeric fields
	case "FilterNumeric1":
		dst.FilterNumeric1 = src.FilterNumeric1
	case "FilterNumeric2":
		dst.FilterNumeric2 = src.FilterNumeric2
	case "FilterNumeric3":
		dst.FilterNumeric3 = src.FilterNumeric3
	case "FilterNumeric4":
		dst.FilterNumeric4 = src.FilterNumeric4
	case "FilterNumeric5":
		dst.FilterNumeric5 = src.FilterNumeric5

	// Text fields
	case "FilterText1":
		dst.FilterText1 = src.FilterText1
	case "FilterText2":
		dst.FilterText2 = src.FilterText2
	case "FilterText3":
		dst.FilterText3 = src.FilterText3
	case "FilterText4":
		dst.FilterText4 = src.FilterText4
	case "FilterText5":
		dst.FilterText5 = src.FilterText5

	// Boolean fields
	case "FilterBoolean1":
		dst.FilterBoolean1 = src.FilterBoolean1
	case "FilterBoolean2":
		dst.FilterBoolean2 = src.FilterBoolean2
	case "FilterBoolean3":
		dst.FilterBoolean3 = src.FilterBoolean3
	case "FilterBoolean4":
		dst.FilterBoolean4 = src.FilterBoolean4
	case "FilterBoolean5":
		dst.FilterBoolean5 = src.FilterBoolean5

	// Enum fields
	case "FilterEnum1":
		dst.FilterEnum1 = src.FilterEnum1
	case "FilterEnum2":
		dst.FilterEnum2 = src.FilterEnum2
	case "FilterEnum3":
		dst.FilterEnum3 = src.FilterEnum3
	case "FilterEnum4":
		dst.FilterEnum4 = src.FilterEnum4
	case "FilterEnum5":
		dst.FilterEnum5 = src.FilterEnum5

	// Location fields
	case "FilterLocation1":
		dst.FilterLocation1 = src.FilterLocation1
	case "FilterLocation2":
		dst.FilterLocation2 = src.FilterLocation2
	case "FilterLocation3":
		dst.FilterLocation3 = src.FilterLocation3
	case "FilterLocation4":
		dst.FilterLocation4 = src.FilterLocation4
	case "FilterLocation5":
		dst.FilterLocation5 = src.FilterLocation5
	}
}

// IsFilterFieldEmpty checks if a filter field is empty (nil or empty string for text types)
func IsFilterFieldEmpty(product *Product, fieldName string) bool {
	switch fieldName {
	// Numeric fields
	case "FilterNumeric1":
		return product.FilterNumeric1 == nil
	case "FilterNumeric2":
		return product.FilterNumeric2 == nil
	case "FilterNumeric3":
		return product.FilterNumeric3 == nil
	case "FilterNumeric4":
		return product.FilterNumeric4 == nil
	case "FilterNumeric5":
		return product.FilterNumeric5 == nil

	// Text fields (nil or empty string)
	case "FilterText1":
		return product.FilterText1 == nil || *product.FilterText1 == ""
	case "FilterText2":
		return product.FilterText2 == nil || *product.FilterText2 == ""
	case "FilterText3":
		return product.FilterText3 == nil || *product.FilterText3 == ""
	case "FilterText4":
		return product.FilterText4 == nil || *product.FilterText4 == ""
	case "FilterText5":
		return product.FilterText5 == nil || *product.FilterText5 == ""

	// Boolean fields
	case "FilterBoolean1":
		return product.FilterBoolean1 == nil
	case "FilterBoolean2":
		return product.FilterBoolean2 == nil
	case "FilterBoolean3":
		return product.FilterBoolean3 == nil
	case "FilterBoolean4":
		return product.FilterBoolean4 == nil
	case "FilterBoolean5":
		return product.FilterBoolean5 == nil

	// Enum fields (nil or empty string)
	case "FilterEnum1":
		return product.FilterEnum1 == nil || *product.FilterEnum1 == ""
	case "FilterEnum2":
		return product.FilterEnum2 == nil || *product.FilterEnum2 == ""
	case "FilterEnum3":
		return product.FilterEnum3 == nil || *product.FilterEnum3 == ""
	case "FilterEnum4":
		return product.FilterEnum4 == nil || *product.FilterEnum4 == ""
	case "FilterEnum5":
		return product.FilterEnum5 == nil || *product.FilterEnum5 == ""

	// Location fields (nil or empty string)
	case "FilterLocation1":
		return product.FilterLocation1 == nil || *product.FilterLocation1 == ""
	case "FilterLocation2":
		return product.FilterLocation2 == nil || *product.FilterLocation2 == ""
	case "FilterLocation3":
		return product.FilterLocation3 == nil || *product.FilterLocation3 == ""
	case "FilterLocation4":
		return product.FilterLocation4 == nil || *product.FilterLocation4 == ""
	case "FilterLocation5":
		return product.FilterLocation5 == nil || *product.FilterLocation5 == ""

	default:
		return true
	}
}

// SetFilterFieldFromDefault sets a filter field to a default value based on field type
func SetFilterFieldFromDefault(product *Product, fieldName string, defaultValue interface{}) bool {
	switch fieldName {
	// Text fields
	case "FilterText1", "FilterText2", "FilterText3", "FilterText4", "FilterText5",
		"FilterEnum1", "FilterEnum2", "FilterEnum3", "FilterEnum4", "FilterEnum5",
		"FilterLocation1", "FilterLocation2", "FilterLocation3", "FilterLocation4", "FilterLocation5":
		if v, ok := defaultValue.(string); ok {
			switch fieldName {
			case "FilterText1":
				product.FilterText1 = &v
			case "FilterText2":
				product.FilterText2 = &v
			case "FilterText3":
				product.FilterText3 = &v
			case "FilterText4":
				product.FilterText4 = &v
			case "FilterText5":
				product.FilterText5 = &v
			case "FilterEnum1":
				product.FilterEnum1 = &v
			case "FilterEnum2":
				product.FilterEnum2 = &v
			case "FilterEnum3":
				product.FilterEnum3 = &v
			case "FilterEnum4":
				product.FilterEnum4 = &v
			case "FilterEnum5":
				product.FilterEnum5 = &v
			case "FilterLocation1":
				product.FilterLocation1 = &v
			case "FilterLocation2":
				product.FilterLocation2 = &v
			case "FilterLocation3":
				product.FilterLocation3 = &v
			case "FilterLocation4":
				product.FilterLocation4 = &v
			case "FilterLocation5":
				product.FilterLocation5 = &v
			}
			return true
		}

	// Boolean fields
	case "FilterBoolean1", "FilterBoolean2", "FilterBoolean3", "FilterBoolean4", "FilterBoolean5":
		if v, ok := defaultValue.(bool); ok {
			switch fieldName {
			case "FilterBoolean1":
				product.FilterBoolean1 = &v
			case "FilterBoolean2":
				product.FilterBoolean2 = &v
			case "FilterBoolean3":
				product.FilterBoolean3 = &v
			case "FilterBoolean4":
				product.FilterBoolean4 = &v
			case "FilterBoolean5":
				product.FilterBoolean5 = &v
			}
			return true
		}

	// Numeric fields
	case "FilterNumeric1", "FilterNumeric2", "FilterNumeric3", "FilterNumeric4", "FilterNumeric5":
		if v, ok := defaultValue.(float64); ok {
			switch fieldName {
			case "FilterNumeric1":
				product.FilterNumeric1 = &v
			case "FilterNumeric2":
				product.FilterNumeric2 = &v
			case "FilterNumeric3":
				product.FilterNumeric3 = &v
			case "FilterNumeric4":
				product.FilterNumeric4 = &v
			case "FilterNumeric5":
				product.FilterNumeric5 = &v
			}
			return true
		}
	}
	return false
}
