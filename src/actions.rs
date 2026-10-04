pub mod arc {
    use gpui_kit::actions;

    actions!(
        arc,
        [
            CopyAsCode,
            DockSidebarLeft,
            DockSidebarRight,
            DockEnvPanelLeft,
            DockEnvPanelRight,
            OpenSettings,
            QuitArc,
            CopySettings,
            ThemeChange,
            ToggleFilePanel,
            ToggleEnvPanel,
            ToggleResponse,
            ToggleCommandPalette,
        ]
    );
}
