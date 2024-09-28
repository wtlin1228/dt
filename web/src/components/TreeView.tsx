import * as React from "react";
import Box from "@mui/material/Box";
import { SimpleTreeView } from "@mui/x-tree-view/SimpleTreeView";
import { TreeItem } from "@mui/x-tree-view/TreeItem";
import { styled } from "@mui/material";

const StyledTreeView = styled(SimpleTreeView)({
  "& .MuiTreeItem-content:hover": {
    backgroundColor: "#ffffff14",
  },
  "& .MuiTreeItem-content.Mui-focused": {
    backgroundColor: "#ffffff1f",
  },
  "& .MuiTreeItem-content.Mui-selected": {
    backgroundColor: "#25303a",
  },
  "& .MuiTreeItem-content.Mui-selected.Mui-focused": {
    backgroundColor: "#90caf947",
  },
});

export function TreeView() {
  return (
    <Box>
      <StyledTreeView>
        <TreeItem itemId="grid" label="Data Grid">
          <TreeItem itemId="grid-community" label="@mui/x-data-grid" />
          <TreeItem itemId="grid-pro" label="@mui/x-data-grid-pro" />
          <TreeItem itemId="grid-premium" label="@mui/x-data-grid-premium" />
        </TreeItem>
        <TreeItem itemId="pickers" label="Date and Time Pickers">
          <TreeItem itemId="pickers-community" label="@mui/x-date-pickers" />
          <TreeItem itemId="pickers-pro" label="@mui/x-date-pickers-pro" />
        </TreeItem>
        <TreeItem itemId="charts" label="Charts">
          <TreeItem itemId="charts-community" label="@mui/x-charts" />
        </TreeItem>
        <TreeItem itemId="tree-view" label="Tree View">
          <TreeItem itemId="tree-view-community" label="@mui/x-tree-view" />
        </TreeItem>
      </StyledTreeView>
    </Box>
  );
}
