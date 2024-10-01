import * as React from "react";
import { Paper, Stack, Typography } from "@mui/material";
import { TreeView } from "./components/TreeView";
import { CustomizedInputBase } from "./components/SearchInput";
import { useQuery } from "@tanstack/react-query";
import { getSearchResult } from "./api";

export const Search = () => {
  const [search, setSearch] = React.useState<undefined | string>();
  const { isPending, isLoading, isError, data, error } = useQuery({
    queryKey: ["search", search],
    queryFn: () => getSearchResult(search!),
    enabled: search !== undefined,
    staleTime: 5 * 1000,
    retry: false,
  });

  return (
    <Stack
      gap={{ xs: 1, md: 2 }}
      sx={{
        padding: {
          xs: 2,
          sm: 3,
          md: 6,
        },
        minHeight: "100vh",
        backgroundColor: "var(--mui-palette-common-background)",
        color: "var(--mui-palette-common-onBackground)",
      }}
    >
      <CustomizedInputBase
        handleSearch={(search) => {
          setSearch(search);
        }}
        isSearching={isLoading}
      />
      <Paper elevation={1} sx={{ padding: 2 }}>
        {isPending ? (
          <Typography>Waiting for input...</Typography>
        ) : isLoading ? (
          <Typography>Searching...</Typography>
        ) : isError ? (
          <Typography>🚨 {error.message}</Typography>
        ) : (
          <TreeView data={data} />
        )}
      </Paper>
    </Stack>
  );
};
