package main

import (
	"database/sql"
	"fmt"
	_ "github.com/mattn/go-sqlite3"
	"log"
	"os"
)

func main() {
	dbPath := "test.db"
	if len(os.Args) > 1 {
		dbPath = os.Args[1]
	}

	db, err := sql.Open("sqlite3", dbPath)
	if err != nil {
		log.Fatal(err)
	}
	defer db.Close()

	rows, err := db.Query("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
	if err != nil {
		log.Fatal(err)
	}
	defer rows.Close()

	fmt.Println("Tables in database:")
	hasCloudStorage := false
	for rows.Next() {
		var name string
		if err := rows.Scan(&name); err != nil {
			log.Fatal(err)
		}
		fmt.Println("-", name)
		if name == "storage_shares" || name == "storage_access_logs" || name == "storage_quotas" {
			hasCloudStorage = true
		}
	}

	if hasCloudStorage {
		fmt.Println("\n✅ CloudStorage tables found!")
	} else {
		fmt.Println("\n❌ CloudStorage tables NOT found (storage_shares, storage_access_logs, storage_quotas)")
	}
}
