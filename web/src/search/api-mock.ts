import { SearchResult } from "./api";

const mockedTraceResult = {
  "i18n.key.pikachu": {
    "/account": {
      // paths from Avatar to /account
      Avatar: [
        // path 1
        [
          {
            module_path: "module/path/can/be/super/long/too/bad/Avatar.tsx",
            symbol_name: "Avatar",
          },
          {
            module_path:
              "module/path/can/be/super/long/too/bad/SuperBigAvatar.tsx",
            symbol_name: "SuperBigAvatar",
          },
          {
            module_path: "module/path/can/be/super/long/too/bad/Header.tsx",
            symbol_name: "Header",
          },
          {
            module_path:
              "module/path/can/be/super/long/too/bad/UserProfileHeader.tsx",
            symbol_name: "UserProfileHeader",
          },
          {
            module_path:
              "module/path/can/be/super/long/too/bad/UserProfile.tsx",
            symbol_name: "UserProfile",
          },
          {
            module_path: "module/path/can/be/super/long/too/bad/Account.tsx",
            symbol_name: "Account",
          },
        ],
        // path 2
        [
          {
            module_path: "module/path/can/be/super/long/too/bad/Avatar.tsx",
            symbol_name: "Avatar",
          },
          {
            module_path: "module/path/can/be/super/long/too/bad/FriendList.tsx",
            symbol_name: "FriendList",
          },
          {
            module_path:
              "module/path/can/be/super/long/too/bad/UserProfile.tsx",
            symbol_name: "UserProfile",
          },
          {
            module_path: "module/path/can/be/super/long/too/bad/Account.tsx",
            symbol_name: "Account",
          },
        ],
      ],
    },
  },
  "i18n.key.pikapi": {
    "/home": {
      // paths from Header to /home
      Header: [
        // path 1
        [
          {
            module_path: "module/path/can/be/super/long/too/bad/Header.tsx",
            symbol_name: "Header",
          },
          {
            module_path: "module/path/can/be/super/long/too/bad/Layout.tsx",
            symbol_name: "Layout",
          },
          {
            module_path: "module/path/can/be/super/long/too/bad/Home.tsx",
            symbol_name: "Home",
          },
        ],
      ],
    },
    "/account": {
      // paths from Header to /account
      Header: [
        // path 1
        [
          {
            module_path: "module/path/can/be/super/long/too/bad/Header.tsx",
            symbol_name: "Header",
          },
          {
            module_path:
              "module/path/can/be/super/long/too/bad/UserProfileHeader.tsx",
            symbol_name: "UserProfileHeader",
          },
          {
            module_path:
              "module/path/can/be/super/long/too/bad/UserProfile.tsx",
            symbol_name: "UserProfile",
          },
          {
            module_path: "module/path/can/be/super/long/too/bad/Account.tsx",
            symbol_name: "Account",
          },
        ],
      ],
    },
  },
};

export const mockedSearchResult: SearchResult = {
  project_root: "module/path/can/be/super/long/too/bad/",
  trace_result: mockedTraceResult,
};
