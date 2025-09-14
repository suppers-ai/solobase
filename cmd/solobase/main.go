package main

import (
	"log"

	"github.com/suppers-ai/solobase"
)

func main() {
	app := solobase.New()
	if err := app.Initialize(); err != nil {
		log.Fatal("Failed to initialize app:", err)
	}
	if err := app.Start(); err != nil {
		log.Fatal("Failed to start app:", err)
	}
}
