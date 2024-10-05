import * as React from "react";
import Paper from "@mui/material/Paper";
import InputBase from "@mui/material/InputBase";
import IconButton from "@mui/material/IconButton";
import SearchIcon from "@mui/icons-material/Search";
import BlurOffIcon from "@mui/icons-material/BlurOff";
import BlurOnIcon from "@mui/icons-material/BlurOn";

export function CustomizedInputBase({
  handleSearch,
  isSearching,
  exactMatch,
  setExactMatch,
}: {
  handleSearch: (search: string) => void;
  isSearching: boolean;
  exactMatch: boolean;
  setExactMatch: (b: boolean) => void;
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
        placeholder="Search Translation"
        inputProps={{ "aria-label": "search translation" }}
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
      <IconButton
        type="button"
        sx={{ p: "10px" }}
        aria-label="toggle match whole word"
        onClick={() => {
          setExactMatch(!exactMatch);
        }}
      >
        {exactMatch ? <BlurOffIcon /> : <BlurOnIcon />}
      </IconButton>
    </Paper>
  );
}
