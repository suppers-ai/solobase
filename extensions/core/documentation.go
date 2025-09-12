package core

import "net/http"

// ExtensionDocumentation provides comprehensive documentation for an extension
type ExtensionDocumentation struct {
	Overview      string           `json:"overview"`
	DataCollected []DataPoint      `json:"data_collected"`
	Endpoints     []EndpointDoc    `json:"endpoints"`
	UsageExamples []UsageExample   `json:"usage_examples"`
	Configuration ConfigDoc        `json:"configuration"`
	Permissions   []PermissionDoc  `json:"permissions"`
	Screenshots   []Screenshot     `json:"screenshots"`
	FAQ           []FAQItem        `json:"faq"`
	Changelog     []ChangelogEntry `json:"changelog"`
}

// DataPoint describes data collected by the extension
type DataPoint struct {
	Name        string   `json:"name"`
	Type        string   `json:"type"`
	Description string   `json:"description"`
	Purpose     string   `json:"purpose"`
	Retention   string   `json:"retention"`
	Sensitive   bool     `json:"sensitive"`
	Examples    []string `json:"examples,omitempty"`
}

// EndpointDoc documents an API endpoint
type EndpointDoc struct {
	Path        string           `json:"path"`
	Methods     []string         `json:"methods"`
	Description string           `json:"description"`
	Parameters  []ParameterDoc   `json:"parameters"`
	Headers     []HeaderDoc      `json:"headers"`
	Request     *RequestExample  `json:"request,omitempty"`
	Response    *ResponseExample `json:"response,omitempty"`
	Errors      []ErrorDoc       `json:"errors"`
	RateLimit   string           `json:"rate_limit,omitempty"`
	Auth        string           `json:"auth"`
}

// ParameterDoc documents a parameter
type ParameterDoc struct {
	Name        string `json:"name"`
	Type        string `json:"type"`
	Required    bool   `json:"required"`
	Description string `json:"description"`
	Default     string `json:"default,omitempty"`
	Example     string `json:"example"`
	Validation  string `json:"validation,omitempty"`
}

// HeaderDoc documents a header
type HeaderDoc struct {
	Name        string `json:"name"`
	Required    bool   `json:"required"`
	Description string `json:"description"`
	Example     string `json:"example"`
}

// RequestExample provides a request example
type RequestExample struct {
	ContentType string      `json:"content_type"`
	Body        interface{} `json:"body"`
	CURL        string      `json:"curl,omitempty"`
}

// ResponseExample provides a response example
type ResponseExample struct {
	StatusCode  int         `json:"status_code"`
	ContentType string      `json:"content_type"`
	Body        interface{} `json:"body"`
}

// ErrorDoc documents possible errors
type ErrorDoc struct {
	StatusCode  int    `json:"status_code"`
	Code        string `json:"code"`
	Description string `json:"description"`
}

// UsageExample provides usage examples
type UsageExample struct {
	Title       string `json:"title"`
	Description string `json:"description"`
	Code        string `json:"code"`
	Language    string `json:"language"`
	Output      string `json:"output,omitempty"`
}

// ConfigDoc documents configuration options
type ConfigDoc struct {
	Description string          `json:"description"`
	Options     []ConfigOption  `json:"options"`
	Examples    []ConfigExample `json:"examples"`
}

// ConfigOption documents a configuration option
type ConfigOption struct {
	Key         string      `json:"key"`
	Type        string      `json:"type"`
	Required    bool        `json:"required"`
	Default     interface{} `json:"default"`
	Description string      `json:"description"`
	Validation  string      `json:"validation,omitempty"`
	Examples    []string    `json:"examples,omitempty"`
}

// ConfigExample provides configuration examples
type ConfigExample struct {
	Name        string      `json:"name"`
	Description string      `json:"description"`
	Config      interface{} `json:"config"`
}

// PermissionDoc documents required permissions
type PermissionDoc struct {
	Name        string   `json:"name"`
	Description string   `json:"description"`
	Resource    string   `json:"resource"`
	Actions     []string `json:"actions"`
	Required    bool     `json:"required"`
	Reason      string   `json:"reason"`
}

// Screenshot provides extension screenshots
type Screenshot struct {
	Title       string `json:"title"`
	Description string `json:"description"`
	URL         string `json:"url"`
	Thumbnail   string `json:"thumbnail,omitempty"`
}

// FAQItem represents a frequently asked question
type FAQItem struct {
	Question string `json:"question"`
	Answer   string `json:"answer"`
	Category string `json:"category,omitempty"`
}

// ChangelogEntry represents a changelog entry
type ChangelogEntry struct {
	Version  string   `json:"version"`
	Date     string   `json:"date"`
	Changes  []string `json:"changes"`
	Breaking bool     `json:"breaking"`
}

// ExtensionWithDashboard extends the Extension interface with dashboard requirements
type ExtensionWithDashboard interface {
	Extension

	// Dashboard returns the dashboard handler for the extension
	DashboardHandler() http.HandlerFunc

	// DashboardPath returns the path for the dashboard (relative to /ext/{name}/)
	DashboardPath() string

	// Documentation returns comprehensive documentation
	Documentation() ExtensionDocumentation
}

// DashboardMetadata provides metadata about an extension's dashboard
type DashboardMetadata struct {
	Title       string   `json:"title"`
	Description string   `json:"description"`
	Icon        string   `json:"icon"`
	Features    []string `json:"features"`
	Preview     string   `json:"preview,omitempty"`
}
