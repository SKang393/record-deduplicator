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

on panelOperation(processRef, panelButton, operation, argumentValue)
  tell application "System Events"
    repeat with appWindow in (every window of processRef)
      set containers to {contents of appWindow}
      try
        repeat with appSheet in (every sheet of appWindow)
          set end of containers to contents of appSheet
        end repeat
      end try
      repeat with containerRef in containers
        set theContainer to contents of containerRef
        if (my controlNamed(theContainer, "AXButton", panelButton, "check", "")) is "OK" and (my controlNamed(theContainer, "AXButton", "Cancel", "check", "")) is "OK" then
          if operation is "check" then return "OK:panel"
          if operation is "press" then return my controlNamed(theContainer, "AXButton", panelButton, "press", "")
          if operation is "save_name" then return my controlNamed(theContainer, "AXTextField", "deduplicator-project.json", "replace", argumentValue)
          if operation is "export_name" then return my controlNamed(theContainer, "AXTextField", "deduplicated-records", "replace", argumentValue)
          if operation is "goto" then
            set frontmost of processRef to true
            keystroke "g" using {command down, shift down}
            delay 0.5
            keystroke argumentValue
            key code 36
            return "OK:goto_sent"
          end if
        end if
      end repeat
    end repeat
  end tell
  return "WAIT:native_panel"
end panelOperation

on clipped(valueToClip)
  try
    set resultText to valueToClip as text
    if (length of resultText) > 160 then return (text 1 thru 160 of resultText) & "..."
    return resultText
  on error
    return "<unavailable>"
  end try
end clipped

on dumpWindows(processRef)
  set outputText to "AX diagnostic (maximum two windows and 150 controls per window)" & linefeed
  tell application "System Events"
    set allWindows to every window of processRef
    set outputText to outputText & "window_count=" & (count of allWindows) & linefeed
    set windowIndex to 0
    repeat with appWindow in allWindows
      set windowIndex to windowIndex + 1
      if windowIndex > 2 then exit repeat
      set outputText to outputText & "window=" & windowIndex & linefeed
      set elementIndex to 0
      try
        set candidates to entire contents of appWindow
        repeat with candidate in candidates
          set elementIndex to elementIndex + 1
          if elementIndex > 150 then exit repeat
          set elementRole to "<unavailable>"
          set elementName to "<unavailable>"
          set elementDescription to "<unavailable>"
          try
            set elementRole to my clipped(role of candidate)
          end try
          try
            set elementName to my clipped(name of candidate)
          end try
          try
            set elementDescription to my clipped(description of candidate)
          end try
          set outputText to outputText & "role=" & elementRole & " name=" & elementName & " description=" & elementDescription & linefeed
        end repeat
      on error errorMessage
        set outputText to outputText & "contents unavailable: " & (my clipped(errorMessage)) & linefeed
      end try
    end repeat
  end tell
  return outputText
end dumpWindows

on run argv
  if (count of argv) is not 5 then error "invalid probe arguments"
  set wantedPid to (item 1 of argv) as integer
  set wantedBundle to item 2 of argv
  set operation to item 3 of argv
  set target to item 4 of argv
  set extra to item 5 of argv
  tell application "System Events"
    set matchingProcesses to every application process whose unix id is wantedPid
    if (count of matchingProcesses) is not 1 then return "WAIT:app_process"
    set processRef to item 1 of matchingProcesses
    if (bundle identifier of processRef as text) is not wantedBundle then error "app PID bundle identifier mismatch"
    if operation is "dump" then return my dumpWindows(processRef)
    set frontmost of processRef to true
    if (count of windows of processRef) is 0 then return "WAIT:app_window"
    if operation is "window" then return "OK:window"
    if operation is "click" or operation is "text" then
      repeat with appWindow in (every window of processRef)
        if operation is "click" then
          set matchResult to my controlNamed(contents of appWindow, "AXButton", target, "press", "")
        else
          set matchResult to my controlNamed(contents of appWindow, "AXStaticText", target, "check", "")
        end if
        if matchResult is "OK" then return "OK:control"
      end repeat
      return "WAIT:control"
    end if
    if operation is "panel_check" then return my panelOperation(processRef, target, "check", "")
    if operation is "panel_goto" then return my panelOperation(processRef, target, "goto", extra)
    if operation is "panel_press" then return my panelOperation(processRef, target, "press", "")
    if operation is "panel_save_name" then return my panelOperation(processRef, target, "save_name", extra)
    if operation is "panel_export_name" then return my panelOperation(processRef, target, "export_name", extra)
  end tell
  error "unsupported probe operation"
end run
