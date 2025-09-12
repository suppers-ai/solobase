module github.com/suppers-ai/auth

go 1.21

require (
	github.com/google/uuid v1.6.0
	github.com/gorilla/sessions v1.2.2
	github.com/lib/pq v1.10.9
	github.com/suppers-ai/database v0.0.0
	github.com/suppers-ai/mailer v0.0.0
	github.com/volatiletech/authboss/v3 v3.5.0
	golang.org/x/crypto v0.18.0
	gorm.io/gorm v1.25.12
)

replace (
	github.com/suppers-ai/database => ../database
	github.com/suppers-ai/mailer => ../mailer
)

require (
	github.com/friendsofgo/errors v0.9.2 // indirect
	github.com/golang/protobuf v1.5.3 // indirect
	github.com/gorilla/securecookie v1.1.2 // indirect
	github.com/jackc/pgpassfile v1.0.0 // indirect
	github.com/jackc/pgservicefile v0.0.0-20221227161230-091c0ba34f0a // indirect
	github.com/jackc/pgx/v5 v5.5.5 // indirect
	github.com/jackc/puddle/v2 v2.2.1 // indirect
	github.com/jinzhu/inflection v1.0.0 // indirect
	github.com/jinzhu/now v1.1.5 // indirect
	github.com/mattn/go-sqlite3 v1.14.22 // indirect
	golang.org/x/net v0.17.0 // indirect
	golang.org/x/oauth2 v0.6.0 // indirect
	golang.org/x/sync v0.1.0 // indirect
	golang.org/x/text v0.14.0 // indirect
	golang.org/x/xerrors v0.0.0-20220907171357-04be3eba64a2 // indirect
	google.golang.org/appengine v1.6.7 // indirect
	google.golang.org/protobuf v1.29.1 // indirect
	gorm.io/driver/postgres v1.5.9 // indirect
	gorm.io/driver/sqlite v1.5.6 // indirect
)
