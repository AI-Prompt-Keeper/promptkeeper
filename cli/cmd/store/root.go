package store

import (
	"github.com/spf13/cobra"
)

var StoreCmd = &cobra.Command{
	Use:   "store",
	Short: "Store secrets (keys, prompts)",
	Long:  "Store provider API keys or prompt templates in the Secure AI Gateway.",
}

