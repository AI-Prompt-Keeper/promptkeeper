package cmd

import (
	"github.com/spf13/cobra"
)

// switchCmd mirrors `workspace switch` for shorter examples (e.g. `prke switch default`).
var switchCmd = &cobra.Command{
	Use:   "switch <name_or_id>",
	Short: "Set the active workspace (alias for workspace switch)",
	Args:  cobra.ExactArgs(1),
	RunE: func(_ *cobra.Command, args []string) error {
		return doWorkspaceSwitch(args[0])
	},
}

func init() {
	rootCmd.AddCommand(switchCmd)
}
