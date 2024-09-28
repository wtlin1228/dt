import * as React from "react";
import CssBaseline from "@mui/material/CssBaseline";
import { ThemeProvider, createTheme } from "@mui/material/styles";
import { Paper, Stack, Typography } from "@mui/material";

import { TreeView } from "./components/TreeView";
import { CustomizedInputBase } from "./components/SearchInput";

const theme = createTheme({
  cssVariables: true,
  colorSchemes: {
    dark: true,
  },
});

export default function App() {
  const [search, setSearch] = React.useState<undefined | string>();
  const [searchResult, setSearchResult] = React.useState<undefined | string>();
  const [isLoading, setIsLoading] = React.useState<boolean>(false);
  const [error, setError] = React.useState<undefined | string>();

  React.useEffect(() => {
    if (!search) {
      return;
    }

    setIsLoading(true);
    setError(undefined);

    const controller = new AbortController();
    const signal = controller.signal;

    fetch(`http://127.0.0.1:8080/search/${search}`, { signal })
      .then((res) => {
        if (res.ok) {
          return res.json();
        } else {
          if (res.status === 404) {
            // Not found
            setError(`${search} not found`);
          } else {
            alert("Have you started the API server?");
          }
        }
      })
      .then((data) => {
        setSearchResult(data);
      })
      .finally(() => {
        setIsLoading(false);
      });

    return () => {
      controller.abort();
    };
  }, [search]);

  return (
    <React.Fragment>
      <CssBaseline />
      <ThemeProvider theme={theme}>
        <Stack
          gap={{ xs: 2, md: 3 }}
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
            {isLoading ? (
              <Typography>Searching...</Typography>
            ) : error ? (
              <Typography>🚨 {error}</Typography>
            ) : (
              // TODO: pass search result into tree view for display
              <TreeView />
            )}
          </Paper>
        </Stack>
      </ThemeProvider>
    </React.Fragment>
  );
}
