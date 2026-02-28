package system

import (
	"github.com/suppers-ai/solobase/core/apptime"
)

// startTime tracks when the application started.
var startTime = apptime.NowTime()

// NavItem represents a navigation item for the admin sidebar.
type NavItem struct {
	Title     string `json:"title"`
	Href      string `json:"href"`
	Icon      string `json:"icon"`
	Separator bool   `json:"separator,omitempty"` // render a separator line before this item
}
