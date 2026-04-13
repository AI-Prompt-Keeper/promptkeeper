package cmd

import (
	"fmt"
	"os"
	"strings"

	"github.com/promptkeeper/cli/internal/api"
	"github.com/promptkeeper/cli/internal/config"
	"github.com/promptkeeper/cli/internal/ui"
	"github.com/promptkeeper/cli/internal/validate"
	"github.com/spf13/cobra"
)

var keyCmd = &cobra.Command{
	Use:   "key",
	Short: "CLI-only key aliases (per workspace)",
}

var keyAliasCmd = &cobra.Command{
	Use:   "alias [old_alias] [new_alias]",
	Short: "Rename a stored key alias for the active workspace",
	Long: `Renames a key alias in local storage for the active workspace.

With no arguments, both names are prompted. With one argument (old alias), only the new name is prompted.`,
	Args: cobra.MaximumNArgs(2),
	RunE: runKeyAlias,
}

var useCmd = &cobra.Command{
	Use:   "use [alias_or_raw_client_key]",
	Short: "Set the default client key for the active workspace",
	Long: `If you pass a pk_mgt_live_ or pk_exe_live_ key, it is verified against POST /v1/auth/verify-client-key for the active workspace, then stored (or kept in-memory for this session only if secure storage is unavailable). If you pass an alias, it must already exist for this workspace (from a prior set).

With no arguments, an interactive wizard prompts for the alias or key.`,
	Args: cobra.MaximumNArgs(1),
	RunE: runUse,
}

func init() {
	rootCmd.AddCommand(keyCmd)
	rootCmd.AddCommand(useCmd)
	keyCmd.AddCommand(keyAliasCmd)
}

func runKeyAlias(_ *cobra.Command, args []string) error {
	var oldA, newA string
	switch len(args) {
	case 2:
		oldA = strings.TrimSpace(args[0])
		newA = strings.TrimSpace(args[1])
	case 1:
		oldA = strings.TrimSpace(args[0])
		if err := ui.FormKeyAliasNewAlias(&newA); err != nil {
			return err
		}
		newA = strings.TrimSpace(newA)
	case 0:
		if err := ui.FormKeyAliasRename(&oldA, &newA); err != nil {
			return err
		}
		oldA = strings.TrimSpace(oldA)
		newA = strings.TrimSpace(newA)
	default:
		return fmt.Errorf("at most 2 arguments allowed")
	}
	if err := validate.ValidateKeyAlias(oldA); err != nil {
		PrintFriendlyError(os.Stderr, "Invalid alias.", err.Error(), []string{bin() + " key alias old new"})
		return err
	}
	if err := validate.ValidateKeyAlias(newA); err != nil {
		PrintFriendlyError(os.Stderr, "Invalid alias.", err.Error(), []string{bin() + " key alias old new"})
		return err
	}
	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	ws := cfg.CurrentWorkspaceID()
	if ws == "" {
		PrintAPIError(os.Stderr, "no active workspace", bin()+" workspace list && "+bin()+" workspace switch <name_or_id>")
		return fmt.Errorf("no active workspace")
	}
	if err := cfg.RenameAliasKey(ws, oldA, newA); err != nil {
		PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	fmt.Fprintln(os.Stdout, ui.SuccessMessage("Alias renamed."))
	return nil
}

func runUse(_ *cobra.Command, args []string) error {
	var arg string
	if len(args) >= 1 {
		arg = strings.TrimSpace(args[0])
	} else {
		if err := ui.FormUseClientKey(&arg); err != nil {
			return err
		}
		arg = strings.TrimSpace(arg)
	}
	if arg == "" {
		PrintFriendlyError(os.Stderr, "Alias or key is required.", "", []string{bin() + " use my_alias"})
		return fmt.Errorf("missing alias or key")
	}
	cfg, err := config.New(rootDebug && rootUseLocalConfig)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "")
		return err
	}
	ws := cfg.CurrentWorkspaceID()
	if ws == "" {
		PrintAPIError(os.Stderr, "no active workspace", bin()+" workspace list && "+bin()+" workspace switch <name_or_id>")
		return fmt.Errorf("no active workspace")
	}
	baseURL := cfg.BaseURL()
	if rootDebug {
		fmt.Fprintln(os.Stderr, ui.DebugLine("base URL: %s", baseURL))
	}

	// Existing alias: just select it as default.
	if !strings.HasPrefix(arg, "pk_mgt_live_") && !strings.HasPrefix(arg, "pk_exe_live_") {
		if cfg.GetAliasKey(ws, arg) == "" {
			PrintAPIError(os.Stderr, "unknown alias "+arg, "Pass a raw pk_mgt_live_ / pk_exe_live_ key (verified for this workspace), or create an alias after storing a key.")
			return fmt.Errorf("unknown alias")
		}
		if err := cfg.SetDefaultKeyAlias(ws, arg); err != nil {
			return err
		}
		fmt.Fprintln(os.Stdout, ui.SuccessMessage("Default key alias for this workspace: "+arg))
		return nil
	}

	client := api.NewClient(baseURL, "")
	if rootDebug {
		client.DebugLog = os.Stderr
	}
	wsID := ws
	out, err := client.VerifyClientKey(arg, &wsID)
	if err != nil {
		PrintAPIError(os.Stderr, err.Error(), "Key must belong to the active workspace (see verify-client-key).")
		return err
	}
	if out.Scope == "mgt" {
		cfg.SetWorkspaceManagementKey(ws, arg)
	} else {
		cfg.SetWorkspaceExecutionKey(ws, arg)
	}
	if config.SecureStorageSupported() {
		cfg.SetAliasKey(ws, "default", arg)
		if err := cfg.SetDefaultKeyAlias(ws, "default"); err != nil {
			return err
		}
		fmt.Fprintln(os.Stdout, ui.SuccessMessage("Stored client key under alias \"default\" and set as active."))
		return nil
	}
	config.SetSessionOverrideClientKey(ws, arg)
	fmt.Fprintln(os.Stderr, ui.WarningBlock("In-memory key only",
		"No OS secure storage — the key is kept for this process only.",
		"It will not persist after the CLI exits."))
	fmt.Fprintln(os.Stdout, ui.SuccessMessage("Using provided key for this session ("+out.Scope+")."))
	return nil
}
