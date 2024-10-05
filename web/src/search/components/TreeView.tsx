import * as React from "react";
import { v4 as uuidv4 } from "uuid";
import { styled } from "@mui/material";
import Box from "@mui/material/Box";
import { SimpleTreeView } from "@mui/x-tree-view/SimpleTreeView";
import { TreeItem } from "@mui/x-tree-view/TreeItem";
import type { ModuleSymbol } from "../shared/type";
import { SearchResult } from "../api";

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

const mapAllModuleSymbolToString = (
  project_root: string,
  tracePaths: ModuleSymbol[][]
): string[][] => {
  return tracePaths.map((tracePath) =>
    tracePath.map(({ module_path, symbol_name }) => {
      let shorterPath = module_path.slice(project_root.length);
      return `${symbol_name}@${shorterPath}`;
    })
  );
};

// Use the same tracePaths references with incremental depth to avoid memory waste.
function TracePathToTreeView({
  tracePaths,
  depth = 0,
}: {
  tracePaths: string[][];
  depth?: number;
}) {
  const groups = new Map<string, string[][]>();
  const group2MaxLength = new Map<string, number>();
  for (let tracePath of tracePaths) {
    if (depth < tracePath.length) {
      const key = tracePath[depth];
      if (groups.has(key)) {
        groups.get(key)!.push(tracePath);
        group2MaxLength.set(
          key,
          Math.max(group2MaxLength.get(key)!, tracePath.length)
        );
      } else {
        groups.set(key, [tracePath]);
        group2MaxLength.set(key, tracePath.length);
      }
    }
  }

  if (groups.size === 0) {
    return;
  }

  const treeItems = [];
  for (let [key, value] of groups) {
    const uniuqueKey = uuidv4();
    treeItems.push(
      <TreeItem key={uniuqueKey} itemId={uniuqueKey} label={key}>
        {depth + 1 < group2MaxLength.get(key)! ? (
          <TracePathToTreeView tracePaths={value} depth={depth + 1} />
        ) : null}
      </TreeItem>
    );
  }
  return treeItems;
}

export const TreeView = React.memo(function TreeView({
  data,
}: {
  data: SearchResult;
}) {
  return (
    <Box
      sx={{
        "overflow-x": "scroll",
        pb: 2,
      }}
    >
      <StyledTreeView>
        {Object.entries(data.trace_result).map(
          ([i18nKey, urlToTraceTargets]) => (
            <TreeItem key={i18nKey} itemId={i18nKey} label={i18nKey}>
              {Object.entries(urlToTraceTargets).map(([url, traceTargets]) => (
                <TreeItem
                  key={`${i18nKey} - ${url}`}
                  itemId={`${i18nKey} - ${url}`}
                  label={url}
                >
                  {Object.entries(traceTargets).map(([traceTarget, paths]) => (
                    <TracePathToTreeView
                      key={`${i18nKey} - ${url} - ${traceTarget}`}
                      tracePaths={mapAllModuleSymbolToString(
                        data.project_root,
                        paths
                      )}
                    />
                  ))}
                </TreeItem>
              ))}
            </TreeItem>
          )
        )}
      </StyledTreeView>
    </Box>
  );
});
