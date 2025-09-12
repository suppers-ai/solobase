# Database Package

A flexible Go database package that provides a universal interface for database operations with support for multiple database drivers.

## Features

- Universal `Database` interface for consistent operations across different databases
- PostgreSQL implementation included
- Connection pooling configuration
- Transaction support
- Prepared statements
- Context-aware operations

## Installation

```bash
go get github.com/suppers-ai/builder/go/packages/database
```

## Usage

### Basic Example

```go
package main

import (
    "context"
    "log"
    "time"
    
    "github.com/suppers-ai/builder/go/packages/database"
)

func main() {
    // Create a new database instance
    db, err := database.New("postgres")
    if err != nil {
        log.Fatal(err)
    }
    defer db.Close()
    
    // Configure connection
    config := database.Config{
        Driver:   "postgres",
        Host:     "localhost",
        Port:     5432,
        Database: "mydb",
        Username: "user",
        Password: "password",
        SSLMode:  "disable",
        MaxOpenConns:    10,
        MaxIdleConns:    5,
        ConnMaxLifetime: time.Hour,
    }
    
    // Connect to database
    ctx := context.Background()
    if err := db.Connect(ctx, config); err != nil {
        log.Fatal(err)
    }
    
    // Execute queries
    rows, err := db.Query(ctx, "SELECT id, name FROM users WHERE active = $1", true)
    if err != nil {
        log.Fatal(err)
    }
    defer rows.Close()
    
    for rows.Next() {
        var id int
        var name string
        if err := rows.Scan(&id, &name); err != nil {
            log.Fatal(err)
        }
        log.Printf("User: %d - %s\n", id, name)
    }
}
```

### Transactions

```go
// Begin transaction
tx, err := db.BeginTx(ctx)
if err != nil {
    log.Fatal(err)
}
defer tx.Rollback() // Will be no-op if committed

// Execute operations within transaction
_, err = tx.Exec(ctx, "INSERT INTO users (name) VALUES ($1)", "John")
if err != nil {
    log.Fatal(err)
}

// Commit transaction
if err := tx.Commit(); err != nil {
    log.Fatal(err)
}
```

### Prepared Statements

```go
// Prepare statement
stmt, err := db.Prepare(ctx, "SELECT name FROM users WHERE id = $1")
if err != nil {
    log.Fatal(err)
}
defer stmt.Close()

// Execute prepared statement multiple times
for _, id := range []int{1, 2, 3} {
    row := stmt.QueryRow(ctx, id)
    var name string
    if err := row.Scan(&name); err != nil {
        log.Printf("Error getting user %d: %v\n", id, err)
        continue
    }
    log.Printf("User %d: %s\n", id, name)
}
```

## Supported Databases

- PostgreSQL (implemented)
- MySQL (planned)
- SQLite (planned)
- MongoDB (planned)

## Configuration Options

| Field | Type | Description |
|-------|------|-------------|
| `Driver` | string | Database driver name (e.g., "postgres") |
| `Host` | string | Database host address |
| `Port` | int | Database port |
| `Database` | string | Database name |
| `Username` | string | Database username |
| `Password` | string | Database password |
| `SSLMode` | string | SSL mode (postgres: disable/require/verify-ca/verify-full) |
| `MaxOpenConns` | int | Maximum number of open connections |
| `MaxIdleConns` | int | Maximum number of idle connections |
| `ConnMaxLifetime` | time.Duration | Maximum connection lifetime |
| `Extra` | map[string]interface{} | Additional driver-specific parameters |

## Adding New Database Drivers

To add support for a new database:

1. Create a new file (e.g., `mysql.go`)
2. Implement the `Database` interface
3. Add the driver case to the `New()` function in `database.go`

Example structure:

```go
type MySQL struct {
    db     *sql.DB
    config Config
}

func NewMySQL() *MySQL {
    return &MySQL{}
}

func (m *MySQL) Connect(ctx context.Context, config Config) error {
    // Implementation
}

// Implement all other Database interface methods...
```