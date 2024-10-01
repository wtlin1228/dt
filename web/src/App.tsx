import * as React from "react";
import CssBaseline from "@mui/material/CssBaseline";
import { ThemeProvider, createTheme } from "@mui/material/styles";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Search } from "./search";

const theme = createTheme({
  cssVariables: true,
  colorSchemes: {
    dark: true,
  },
});

const queryClient = new QueryClient();

export default function App() {
  return (
    <React.Fragment>
      <CssBaseline />
      <ThemeProvider theme={theme}>
        <QueryClientProvider client={queryClient}>
          <Search />
        </QueryClientProvider>
      </ThemeProvider>
    </React.Fragment>
  );
}
