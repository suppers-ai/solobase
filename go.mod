module github.com/suppers-ai/solobase

go 1.25.0

require (
	github.com/glebarez/go-sqlite v1.21.2
	github.com/google/uuid v1.6.0
	github.com/gorilla/mux v1.8.1
	github.com/lib/pq v1.10.9
	github.com/microcosm-cc/bluemonday v1.0.27
	github.com/rhnvrm/simples3 v0.11.0
	github.com/stretchr/testify v1.11.1
	go.bytecodealliance.org/cm v0.1.0
	golang.org/x/crypto v0.46.0
	golang.org/x/oauth2 v0.30.0
	gopkg.in/yaml.v3 v3.0.1
)

require (
	cloud.google.com/go/compute/metadata v0.3.0 // indirect
	github.com/aymerick/douceur v0.2.0 // indirect
	github.com/cvilsmeier/sqinn-go v1.2.0 // indirect
	github.com/davecgh/go-spew v1.1.1 // indirect
	github.com/dustin/go-humanize v1.0.1 // indirect
	github.com/golang/protobuf v1.5.3 // indirect
	github.com/gorilla/css v1.0.1 // indirect
	github.com/kr/pretty v0.3.0 // indirect
	github.com/mattn/go-isatty v0.0.17 // indirect
	github.com/pmezard/go-difflib v1.0.0 // indirect
	github.com/remyoudompheng/bigfft v0.0.0-20230129092748-24d4a6f8daec // indirect
	github.com/rogpeppe/go-internal v1.12.0 // indirect
	golang.org/x/net v0.47.0 // indirect
	golang.org/x/sys v0.39.0 // indirect
	google.golang.org/appengine v1.6.7 // indirect
	google.golang.org/protobuf v1.36.6 // indirect
	gopkg.in/check.v1 v1.0.0-20201130134442-10cb98267c6c // indirect
	modernc.org/libc v1.22.5 // indirect
	modernc.org/mathutil v1.5.0 // indirect
	modernc.org/memory v1.5.0 // indirect
	modernc.org/sqlite v1.23.1 // indirect
)

exclude cloud.google.com/go v0.34.0

replace golang.org/x/oauth2 => golang.org/x/oauth2 v0.6.0
