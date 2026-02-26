package main

import (
	"os"

	"github.com/promptkeeper/cli/cmd"
)

func main() {
	if err := cmd.Execute(); err != nil {
		os.Exit(1)
	}
}
