package cmd

import (
	"github.com/spf13/cobra"
)

// switchCmd mirrors `workspace switch` for shorter examples (e.g. `prke switch default`).
var switchCmd = &cobra.Command{
	Use:   "switch [name_or_id]",
	Short: "Set the active workspace (alias for workspace switch)",
	Long:  `Same as "workspace switch". With no arguments, choose a workspace from an interactive list.`,
	Args:  cobra.MaximumNArgs(1),
	RunE: func(_ *cobra.Command, args []string) error {
		if len(args) < 1 {
			return workspaceSwitchInteractive()
		}
		return doWorkspaceSwitch(args[0])
	},
}

func init() {
	rootCmd.AddCommand(switchCmd)
}
