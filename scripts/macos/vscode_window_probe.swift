#!/usr/bin/env swift

import ApplicationServices
import CoreGraphics
import Foundation

private enum ProbeError: Error, CustomStringConvertible {
  case usage(String)
  case invalidArgument(String)
  case notFound(String)
  case permission(String)

  var description: String {
    switch self {
    case .usage(let message), .invalidArgument(let message), .notFound(let message), .permission(let message):
      return message
    }
  }
}

private enum Command: String {
  case id
  case rect
  case move
}

private func usage() -> String {
  return """
  Usage:
    vscode-window-probe id --pid <pid>
    vscode-window-probe rect --pid <pid>
    vscode-window-probe move --pid <pid> --x <int> --y <int> --w <int> --h <int>
  """
}

private func parseCommandAndArgs() throws -> (Command, [String: String]) {
  let args = Array(CommandLine.arguments.dropFirst())
  guard !args.isEmpty else {
    throw ProbeError.usage(usage())
  }

  guard let command = Command(rawValue: args[0]) else {
    throw ProbeError.usage(usage())
  }

  var options: [String: String] = [:]
  var index = 1
  while index < args.count {
    let token = args[index]
    guard token.hasPrefix("--") else {
      throw ProbeError.invalidArgument("Unexpected argument: \(token)")
    }
    guard index + 1 < args.count else {
      throw ProbeError.invalidArgument("Missing value for option: \(token)")
    }
    options[token] = args[index + 1]
    index += 2
  }
  return (command, options)
}

private func parseInt(_ options: [String: String], _ name: String) throws -> Int {
  guard let raw = options[name] else {
    throw ProbeError.invalidArgument("Missing required option: \(name)")
  }
  guard let value = Int(raw) else {
    throw ProbeError.invalidArgument("Invalid integer for \(name): \(raw)")
  }
  return value
}

private func asInt(_ value: CGFloat) -> Int {
  return Int(round(Double(value)))
}

private func cgMainWindowInfo(for pid: pid_t) -> (id: CGWindowID, bounds: CGRect)? {
  guard
    let raw = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]]
  else {
    return nil
  }

  var bestWindowID: CGWindowID?
  var bestBounds: CGRect?
  var bestArea: CGFloat = -1

  for item in raw {
    guard let ownerPid = item[kCGWindowOwnerPID as String] as? Int, ownerPid == Int(pid) else {
      continue
    }
    guard let layer = item[kCGWindowLayer as String] as? Int, layer == 0 else {
      continue
    }
    if let onscreen = item[kCGWindowIsOnscreen as String] as? Int, onscreen == 0 {
      continue
    }
    if let alpha = item[kCGWindowAlpha as String] as? Double, alpha <= 0.01 {
      continue
    }
    guard
      let boundsDict = item[kCGWindowBounds as String] as? [String: CGFloat],
      let x = boundsDict["X"],
      let y = boundsDict["Y"],
      let width = boundsDict["Width"],
      let height = boundsDict["Height"]
    else {
      continue
    }

    if width < 600 || height < 400 {
      continue
    }

    let area = width * height
    if area > bestArea {
      bestArea = area
      bestWindowID = item[kCGWindowNumber as String] as? CGWindowID
      bestBounds = CGRect(x: x, y: y, width: width, height: height)
    }
  }

  guard let windowID = bestWindowID, let bounds = bestBounds else {
    return nil
  }
  return (windowID, bounds)
}

private func copyAXElementAttribute(_ element: AXUIElement, attribute: CFString) -> AXUIElement? {
  var raw: CFTypeRef?
  guard AXUIElementCopyAttributeValue(element, attribute, &raw) == .success else {
    return nil
  }
  guard let raw else {
    return nil
  }
  guard CFGetTypeID(raw) == AXUIElementGetTypeID() else {
    return nil
  }
  return unsafeBitCast(raw, to: AXUIElement.self)
}

private func copyAXValueAttribute(_ element: AXUIElement, attribute: CFString) -> AXValue? {
  var raw: CFTypeRef?
  guard AXUIElementCopyAttributeValue(element, attribute, &raw) == .success else {
    return nil
  }
  guard let raw else {
    return nil
  }
  guard CFGetTypeID(raw) == AXValueGetTypeID() else {
    return nil
  }
  return unsafeBitCast(raw, to: AXValue.self)
}

private func copyAXWindowListFirst(_ element: AXUIElement) -> AXUIElement? {
  var raw: CFTypeRef?
  guard AXUIElementCopyAttributeValue(element, kAXWindowsAttribute as CFString, &raw) == .success else {
    return nil
  }
  guard let raw else {
    return nil
  }
  guard CFGetTypeID(raw) == CFArrayGetTypeID() else {
    return nil
  }
  let array = unsafeBitCast(raw, to: CFArray.self) as NSArray
  guard let first = array.firstObject as AnyObject? else {
    return nil
  }
  guard CFGetTypeID(first) == AXUIElementGetTypeID() else {
    return nil
  }
  return unsafeBitCast(first, to: AXUIElement.self)
}

private func axFrontWindow(for pid: pid_t) -> AXUIElement? {
  let appElement = AXUIElementCreateApplication(pid)

  if let focused = copyAXElementAttribute(appElement, attribute: kAXFocusedWindowAttribute as CFString) {
    return focused
  }

  if let main = copyAXElementAttribute(appElement, attribute: kAXMainWindowAttribute as CFString) {
    return main
  }

  return copyAXWindowListFirst(appElement)
}

private func axWindowRect(_ window: AXUIElement) -> CGRect? {
  guard let positionValue = copyAXValueAttribute(window, attribute: kAXPositionAttribute as CFString) else {
    return nil
  }
  guard AXValueGetType(positionValue) == .cgPoint else {
    return nil
  }
  var point = CGPoint.zero
  guard AXValueGetValue(positionValue, .cgPoint, &point) else {
    return nil
  }

  guard let sizeValue = copyAXValueAttribute(window, attribute: kAXSizeAttribute as CFString) else {
    return nil
  }
  guard AXValueGetType(sizeValue) == .cgSize else {
    return nil
  }
  var size = CGSize.zero
  guard AXValueGetValue(sizeValue, .cgSize, &size) else {
    return nil
  }

  return CGRect(origin: point, size: size)
}

private func setAXWindowRect(_ window: AXUIElement, rect: CGRect) -> Bool {
  var point = rect.origin
  var size = rect.size

  guard let pointValue = AXValueCreate(.cgPoint, &point) else {
    return false
  }
  guard let sizeValue = AXValueCreate(.cgSize, &size) else {
    return false
  }

  let setPos = AXUIElementSetAttributeValue(window, kAXPositionAttribute as CFString, pointValue)
  let setSize = AXUIElementSetAttributeValue(window, kAXSizeAttribute as CFString, sizeValue)
  return setPos == .success && setSize == .success
}

private func printRect(_ rect: CGRect) {
  print("\(asInt(rect.origin.x)),\(asInt(rect.origin.y)),\(asInt(rect.size.width)),\(asInt(rect.size.height))")
}

private func run() throws {
  let (command, options) = try parseCommandAndArgs()
  let pidValue = try parseInt(options, "--pid")
  let pid = pid_t(pidValue)

  switch command {
  case .id:
    guard let info = cgMainWindowInfo(for: pid) else {
      throw ProbeError.notFound("No main CG window found for pid \(pidValue)")
    }
    print(info.id)

  case .rect:
    if let window = axFrontWindow(for: pid), let rect = axWindowRect(window) {
      printRect(rect)
      return
    }
    guard let info = cgMainWindowInfo(for: pid) else {
      throw ProbeError.notFound("No window rect found for pid \(pidValue)")
    }
    printRect(info.bounds)

  case .move:
    let x = try parseInt(options, "--x")
    let y = try parseInt(options, "--y")
    let w = try parseInt(options, "--w")
    let h = try parseInt(options, "--h")

    guard let window = axFrontWindow(for: pid) else {
      throw ProbeError.permission("No AX window for pid \(pidValue) (grant Accessibility permission).")
    }
    let rect = CGRect(x: x, y: y, width: w, height: h)
    guard setAXWindowRect(window, rect: rect) else {
      throw ProbeError.permission("Failed to move window for pid \(pidValue) (grant Accessibility permission).")
    }
  }
}

do {
  try run()
  exit(0)
} catch let error as ProbeError {
  fputs("ERROR: \(error.description)\n", stderr)
  switch error {
  case .usage, .invalidArgument:
    exit(2)
  case .notFound, .permission:
    exit(1)
  }
} catch {
  fputs("ERROR: \(error)\n", stderr)
  exit(1)
}
