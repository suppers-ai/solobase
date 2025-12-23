package main

import (
	"log"
	"os"

	"github.com/suppers-ai/solobase"
	"github.com/suppers-ai/solobase/builds/go/database"
)

func main() {
	// Get database path from environment or use default
	dbPath := os.Getenv("DATABASE_PATH")
	if dbPath == "" {
		dbPath = ".data/solobase.db"
	}

	// Create SQLite database
	db, err := database.NewSQLite(dbPath)
	if err != nil {
		log.Fatal("Failed to create database:", err)
	}
	defer db.Close()

	// Create Solobase app with the database
	app := solobase.NewWithOptions(&solobase.Options{
		Database:     db,
		DatabaseType: "sqlite",
	})

	if err := app.Initialize(); err != nil {
		log.Fatal("Failed to initialize app:", err)
	}
	if err := app.Start(); err != nil {
		log.Fatal("Failed to start app:", err)
	}
}
