on run argv
  set targetPID to (item 1 of argv) as integer
  set reportText to "target-pid=" & targetPID & linefeed
  try
    with timeout of 10 seconds
      tell application "System Events"
        set matchingProcesses to every application process whose unix id is targetPID
        if (count of matchingProcesses) is 0 then return reportText & "process-not-found"
        set targetProcess to item 1 of matchingProcesses
        set processWindows to every window of targetProcess
        set reportText to reportText & "window-count=" & (count of processWindows) & linefeed
        set windowNumber to 0
        repeat with windowRef in processWindows
          set windowNumber to windowNumber + 1
          if windowNumber > 3 then exit repeat
          try
            set windowName to name of windowRef as text
          on error
            set windowName to "<unavailable>"
          end try
          set reportText to reportText & "window=" & windowNumber & " name=" & windowName & linefeed
          try
            set controls to entire contents of windowRef
            set controlNumber to 0
            repeat with controlRef in controls
              set controlNumber to controlNumber + 1
              if controlNumber > 100 then exit repeat
              try
                set controlRole to role of controlRef as text
              on error
                set controlRole to "<unavailable>"
              end try
              try
                set controlName to name of controlRef as text
              on error
                set controlName to "<unavailable>"
              end try
              if (length of controlName) > 256 then set controlName to text 1 thru 256 of controlName
              set reportText to reportText & "  role=" & controlRole & " name=" & controlName & linefeed
            end repeat
          on error treeError number treeErrorNumber
            set reportText to reportText & "tree-error=" & treeErrorNumber & " " & treeError & linefeed
          end try
        end repeat
      end tell
    end timeout
  on error processError number processErrorNumber
    return reportText & "accessibility-error=" & processErrorNumber & " " & processError
  end try
  return reportText
end run
