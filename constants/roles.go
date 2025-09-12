package constants

// UserRole represents the role of a user
type UserRole string

// User role constants
const (
	RoleUser    UserRole = "user"
	RoleManager UserRole = "manager"
	RoleAdmin   UserRole = "admin"
	RoleDeleted UserRole = "deleted"
)

// String returns the string representation of the role
func (r UserRole) String() string {
	return string(r)
}

// IsValid checks if the role is valid
func (r UserRole) IsValid() bool {
	switch r {
	case RoleUser, RoleManager, RoleAdmin, RoleDeleted:
		return true
	default:
		return false
	}
}

// HasPermission checks if the role has permission for another role
func (r UserRole) HasPermission(required UserRole) bool {
	switch r {
	case RoleAdmin:
		return true // Admin has all permissions
	case RoleManager:
		return required == RoleManager || required == RoleUser
	case RoleUser:
		return required == RoleUser
	default:
		return false
	}
}
