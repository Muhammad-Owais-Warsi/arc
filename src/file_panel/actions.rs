use gpui_kit::actions;

actions!(
    file_panel,
    [
        CreateFile,
        CreateFolder,
        DeleteItem,
        TrashItem,
        StressTestPlayground,
        RenameItem,
        CopyPath,
        CopyRelativePath,
    ]
);
