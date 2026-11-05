-- All operations target the exact child PID launched by probe.py.
-- A WAIT result means no UI action was taken.  Paths arrive as argv, never code.
on controlNamed(rootElement, wantedRole, wantedName, operation, replacement)
  tell application "System Events"
    set candidates to entire contents of rootElement
    repeat with candidate in candidates
      try
        if (role of candidate as text) is wantedRole then
          set namesFound to {}
          try
            set end of namesFound to (name of candidate as text)
          end try
          try
            set end of namesFound to (title of candidate as text)
          end try
          try
            set end of namesFound to (description of candidate as text)
          end try
          try
            set end of namesFound to (value of candidate as text)
          end try
          if namesFound contains wantedName then
            if operation is "press" then
              if not (enabled of candidate) then return "WAIT:disabled"
              perform action "AXPress" of candidate
            else if operation is "replace" then
              set value of candidate to replacement
            end if
            return "OK"
          end if
        end if
      end try
    end repeat
  end tell
  return "WAIT:not_found"
end controlNamed

-- Never act on an ambiguous review/recovery button, even across app windows.
on uniqueButton(processRef, wantedName, alternateName)
  tell application "System Events"
    set matchingButtons to {}
    repeat with appWindow in (every window of processRef)
      set candidates to entire contents of appWindow
      repeat with candidate in candidates
        try
          if (role of candidate as text) is "AXButton" then
            set namesFound to {}
            try
              set end of namesFound to (name of candidate as text)
            end try
            try
              set end of namesFound to (title of candidate as text)
            end try
            try
              set end of namesFound to (description of candidate as text)
            end try
            if namesFound contains wantedName or (alternateName is not "" and namesFound contains alternateName) then set end of matchingButtons to candidate
          end if
        end try
      end repeat
    end repeat
    if (count of matchingButtons) is 0 then return "WAIT:not_found"
    if (count of matchingButtons) is not 1 then return "WAIT:ambiguous"
    set matchedButton to item 1 of matchingButtons
    if not (enabled of matchedButton) then return "WAIT:disabled"
    perform action "AXPress" of matchedButton
    return "OK:unique_button"
  end tell
end uniqueButton

-- Query only controls directly owned by a native panel or one of its first
-- eight immediate groups. Never walk the parent window's WebView descendants.
on directPanelControl(rootElement, wantedRole, wantedName, operation, replacement)
  tell application "System Events"
    set candidates to {}
    try
      if wantedRole is "AXButton" then
        set candidates to every button of rootElement
      else if wantedRole is "AXTextField" then
        set candidates to every text field of rootElement
      end if
    end try
    set checkedCount to 0
    repeat with candidate in candidates
      set checkedCount to checkedCount + 1
      if checkedCount > 40 then exit repeat
      try
        set namesFound to {}
        try
          set end of namesFound to (name of candidate as text)
        end try
        try
          set end of namesFound to (title of candidate as text)
        end try
        try
          set end of namesFound to (description of candidate as text)
        end try
        try
          set end of namesFound to (value of candidate as text)
        end try
        if namesFound contains wantedName then
          if operation is "press" then
            if not (enabled of candidate) then return "WAIT:disabled"
            perform action "AXPress" of candidate
          else if operation is "replace" then
            set value of candidate to replacement
          end if
          return "OK"
        end if
      end try
    end repeat
  end tell
  return "WAIT:not_found"
end directPanelControl

on shallowPanelControl(rootElement, wantedRole, wantedName, operation, replacement)
  set resultText to my directPanelControl(rootElement, wantedRole, wantedName, operation, replacement)
  if resultText is not "WAIT:not_found" then return resultText
  tell application "System Events"
    try
      set groupNumber to 0
      repeat with groupRef in (every group of rootElement)
        set groupNumber to groupNumber + 1
        if groupNumber > 8 then exit repeat
        set resultText to my directPanelControl(contents of groupRef, wantedRole, wantedName, operation, replacement)
        if resultText is not "WAIT:not_found" then return resultText
      end repeat
    end try
  end tell
  return "WAIT:not_found"
end shallowPanelControl

on tryPanel(theContainer, processRef, panelButton, operation, argumentValue)
  tell application "System Events"
    if (my shallowPanelControl(theContainer, "AXButton", panelButton, "check", "")) is "OK" and (my shallowPanelControl(theContainer, "AXButton", "Cancel", "check", "")) is "OK" then
      if operation is "check" then return "OK:panel"
      if operation is "press" then return my shallowPanelControl(theContainer, "AXButton", panelButton, "press", "")
      if operation is "save_name" then
        if argumentValue is "native-ui-project.json" then
          set requestedStem to "native-ui-project"
        else if argumentValue is "recovered-ui-project.json" then
          set requestedStem to "recovered-ui-project"
        else
          return "WAIT:invalid_save_name"
        end if
        set saveNameResult to my shallowPanelControl(theContainer, "AXTextField", "deduplicator-project.json", "replace", argumentValue)
        if saveNameResult is not "WAIT:not_found" then return saveNameResult
        -- AppKit may hide the .json extension in the actual Save As field.
        -- Enter the requested stem and let the native JSON filter add it.
        return my shallowPanelControl(theContainer, "AXTextField", "deduplicator-project", "replace", requestedStem)
      end if
      if operation is "export_name" then return my shallowPanelControl(theContainer, "AXTextField", "deduplicated-records", "replace", argumentValue)
      if operation is "goto" then
        set frontmost of processRef to true
        if not (frontmost of processRef) then return "WAIT:not_frontmost"
        keystroke "g" using {command down, shift down}
        delay 0.5
        if not (frontmost of processRef) then return "WAIT:not_frontmost"
        keystroke argumentValue
        key code 36
        return "OK:goto_sent"
      end if
    end if
  end tell
  return "WAIT:native_panel"
end tryPanel

on panelOperation(processRef, panelButton, operation, argumentValue)
  tell application "System Events"
    set appWindows to every window of processRef
    -- Sheets are the normal Tauri/rfd parented-panel path. Inspect them first.
    repeat with appWindow in appWindows
      try
        repeat with appSheet in (every sheet of appWindow)
          set panelResult to my tryPanel(contents of appSheet, processRef, panelButton, operation, argumentValue)
          if panelResult is not "WAIT:native_panel" then return panelResult
        end repeat
      end try
    end repeat
    -- Some AppKit panels may be exposed as app-owned windows instead.
    repeat with appWindow in appWindows
      set panelResult to my tryPanel(contents of appWindow, processRef, panelButton, operation, argumentValue)
      if panelResult is not "WAIT:native_panel" then return panelResult
    end repeat
  end tell
  return "WAIT:native_panel"
end panelOperation

on clipped(valueToClip)
  try
    set resultText to valueToClip as text
    set safeText to ""
    repeat with oneCharacter in characters of resultText
      if oneCharacter is linefeed or oneCharacter is return or oneCharacter is tab then
        set safeText to safeText & " "
      else
        set safeText to safeText & oneCharacter
      end if
      if (length of safeText) >= 160 then exit repeat
    end repeat
    return safeText
  on error
    return "<unavailable>"
  end try
end clipped

on dumpWindows(processRef)
  set outputText to "AX diagnostic (maximum two windows and eight direct sheets each)" & linefeed
  tell application "System Events"
    set allWindows to every window of processRef
    set outputText to outputText & "window_count=" & (count of allWindows) & linefeed
    set windowIndex to 0
    repeat with appWindow in allWindows
      set windowIndex to windowIndex + 1
      if windowIndex > 2 then exit repeat
      set outputText to outputText & "window=" & windowIndex & linefeed
      try
        set outputText to outputText & "window_name=" & (my clipped(name of appWindow)) & linefeed
      end try
      try
        set sheetsFound to every sheet of appWindow
        set outputText to outputText & "sheet_count=" & (count of sheetsFound) & linefeed
        set sheetNumber to 0
        repeat with sheetRef in sheetsFound
          set sheetNumber to sheetNumber + 1
          if sheetNumber > 8 then exit repeat
          set outputText to outputText & "sheet=" & sheetNumber & linefeed
          try
            set buttonNames to name of every button of sheetRef
            set outputText to outputText & "direct_buttons=" & (my clipped(buttonNames)) & linefeed
          end try
          try
            set outputText to outputText & "direct_groups=" & (count of every group of sheetRef) & linefeed
          end try
        end repeat
      on error errorMessage
        set outputText to outputText & "sheets unavailable: " & (my clipped(errorMessage)) & linefeed
      end try
    end repeat
  end tell
  return outputText
end dumpWindows

on run argv
  if (count of argv) is not 7 then error "invalid probe arguments"
  set wantedPid to (item 1 of argv) as integer
  set wantedBundle to item 2 of argv
  set operation to item 3 of argv
  set target to item 4 of argv
  set extra to item 5 of argv
  set wantedName to item 6 of argv
  set fallbackMode to item 7 of argv
  tell application "System Events"
    set matchingProcesses to every application process whose unix id is wantedPid
    if (count of matchingProcesses) is not 1 then return "WAIT:app_process" & linefeed & "requested_pid=" & wantedPid & linefeed & "observed_pid=<unavailable>" & linefeed & "observed_name=<unavailable>" & linefeed & "observed_bundle=<unavailable>"
    set processRef to item 1 of matchingProcesses
    set observedPid to -1
    set observedName to "<unavailable>"
    set observedBundle to "<unavailable>"
    set bundleState to "read_error"
    set bundleErrorCode to "<none>"
    try
      set observedPid to (unix id of processRef) as integer
    end try
    try
      set observedName to my clipped(name of processRef)
    end try
    try
      set bundleValue to bundle identifier of processRef
      if bundleValue is missing value then
        set bundleState to "missing"
        set observedBundle to "<missing>"
      else
        set observedBundle to my clipped(bundleValue)
        if observedBundle is "" then
          set bundleState to "missing"
          set observedBundle to "<missing>"
        else
          set bundleState to "present"
        end if
      end if
    on error errorMessage number errorNumber
      set bundleState to "read_error"
      set bundleErrorCode to errorNumber as text
    end try
    set identityInfo to linefeed & "requested_pid=" & wantedPid & linefeed & "observed_pid=" & observedPid & linefeed & "observed_name=" & observedName & linefeed & "observed_bundle=" & observedBundle & linefeed & "bundle_state=" & bundleState & linefeed & "bundle_error_code=" & bundleErrorCode
    if observedPid is not wantedPid then return "WAIT:app_process" & identityInfo
    set identityMode to "bundle_match"
    if bundleState is "present" then
      considering case
        if observedBundle is not wantedBundle then return "WAIT:bundle_mismatch" & identityInfo & linefeed & "identity_mode=blocked"
      end considering
    else
      if fallbackMode is not "verified-owned-pid" then return "WAIT:bundle_unavailable" & identityInfo & linefeed & "identity_mode=blocked"
      considering case
        if observedName is not wantedName then return "WAIT:process_name_mismatch" & identityInfo & linefeed & "identity_mode=blocked"
      end considering
      set identityMode to "owned_pid_unavailable_bundle"
    end if
    set identityInfo to identityInfo & linefeed & "identity_mode=" & identityMode
    if operation is "dump" then return (my dumpWindows(processRef)) & identityInfo
    set frontmost of processRef to true
    if (count of windows of processRef) is 0 then return "WAIT:app_window" & identityInfo
    if operation is "window" then return "OK:window" & identityInfo
    if operation is "click_unique" then return (my uniqueButton(processRef, target, extra)) & identityInfo
    if operation is "click" or operation is "text" then
      repeat with appWindow in (every window of processRef)
        if operation is "click" then
          set matchResult to my controlNamed(contents of appWindow, "AXButton", target, "press", "")
        else
          set matchResult to my controlNamed(contents of appWindow, "AXStaticText", target, "check", "")
        end if
        if matchResult is "OK" then return "OK:control" & identityInfo
      end repeat
      return "WAIT:control" & identityInfo
    end if
    if operation is "panel_check" then return (my panelOperation(processRef, target, "check", "")) & identityInfo
    if operation is "panel_goto" then return (my panelOperation(processRef, target, "goto", extra)) & identityInfo
    if operation is "panel_press" then return (my panelOperation(processRef, target, "press", "")) & identityInfo
    if operation is "panel_save_name" then return (my panelOperation(processRef, target, "save_name", extra)) & identityInfo
    if operation is "panel_export_name" then return (my panelOperation(processRef, target, "export_name", extra)) & identityInfo
  end tell
  error "unsupported probe operation"
end run
