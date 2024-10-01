import * as React from "react";
import Paper from "@mui/material/Paper";
import InputBase from "@mui/material/InputBase";
import IconButton from "@mui/material/IconButton";
import SearchIcon from "@mui/icons-material/Search";

export function CustomizedInputBase({
  handleSearch,
  isSearching,
}: {
  handleSearch: (search: string) => void;
  isSearching: boolean;
}) {
  const inputRef = React.useRef<HTMLInputElement>();

  return (
    <Paper
      component="form"
      sx={{
        p: "2px 4px",
        display: "flex",
        alignItems: "center",
      }}
      onSubmit={(e) => {
        e.preventDefault();
        if (isSearching) {
          return;
        }
        if (inputRef.current?.value) {
          handleSearch(inputRef.current?.value);
        }
      }}
    >
      <InputBase
        fullWidth
        sx={{ ml: 1, flex: 1 }}
        placeholder="Search Translation Key"
        inputProps={{ "aria-label": "search translation key" }}
        inputRef={inputRef}
        disabled={isSearching}
      />
      <IconButton
        type="button"
        sx={{ p: "10px" }}
        aria-label="search"
        onClick={() => {
          if (isSearching) {
            return;
          }
          if (inputRef.current?.value) {
            handleSearch(inputRef.current?.value);
          }
        }}
        disabled={isSearching}
      >
        <SearchIcon />
      </IconButton>
    </Paper>
  );
}
