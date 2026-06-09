# Markdown generation utilities (internal)
#
# Functions for generating markdown documentation from lib metadata.
#
{ lib }:
let
  # Generate markdown anchor from lib name
  nameToAnchor = name: builtins.replaceStrings [ "." ] [ "-" ] name;

  # Render a Nix type to a human-readable string
  typeToString =
    type:
    if type == null then
      null
    else if builtins.isString type then
      type
    else if builtins.isAttrs type && type ? description then
      type.description
    else
      builtins.toString type;

  # Pretty-print a Nix value for documentation
  valueToNix =
    v:
    if builtins.isString v then
      ''"${v}"''
    else if builtins.isInt v then
      toString v
    else if builtins.isBool v then
      if v then "true" else "false"
    else if builtins.isNull v then
      "null"
    else if builtins.isList v then
      "[ ${lib.concatMapStringsSep " " valueToNix v} ]"
    else if builtins.isAttrs v then
      "{ ${lib.concatStringsSep " " (lib.mapAttrsToList (k: val: "${k} = ${valueToNix val};") v)} }"
    else
      toString v;

  # Generate index entry for a single lib
  libToIndexEntry =
    name: meta:
    let
      anchor = nameToAnchor name;
      fileLink = if meta.file or null != null then " ([source](${meta.file}))" else "";
    in
    "- [`${name}`](#${anchor})${fileLink}";

  # Render test cases as a markdown table
  testsToMarkdown =
    tests:
    let
      testNames = builtins.attrNames tests;
    in
    if testNames == [ ] then
      ""
    else
      let
        rows = map (
          testName:
          let
            t = tests.${testName};
            argsStr = valueToNix (t.args or { });
            expectedStr =
              if t.expected or null != null then valueToNix t.expected else "*(assertions)*";
          in
          "| ${testName} | `${argsStr}` | `${expectedStr}` |"
        ) testNames;
      in
      ''

        **Tests:**

        | Name | Input | Expected |
        |---|---|---|
        ${lib.concatStringsSep "\n" rows}
      '';

  # Generate markdown for a single lib
  libToMarkdown =
    name: meta:
    let
      anchor = nameToAnchor name;
      visibleStr = if meta.visible or true then "" else " *(private)*";
      descStr = meta.description or "No description";
      typeStr =
        let
          rendered = typeToString (meta.type or null);
        in
        if rendered != null then
          ''

            **Type:** `${rendered}`
          ''
        else
          "";
      fileStr =
        if meta.file or null != null then
          ''

            **Source:** [${meta.file}](${meta.file})
          ''
        else
          "";
      exampleStr =
        if meta.example or null != null then
          ''

            **Example:**
            ```nix
            ${meta.example}
            ```
          ''
        else
          "";
      testStr = testsToMarkdown (meta.tests or { });
    in
    ''
      ### `${name}` {#${anchor}}${visibleStr}
      ${typeStr}
      ${descStr}
      ${fileStr}${exampleStr}${testStr}
    '';

  # Generate full markdown document
  generateMarkdown =
    allLibsMeta:
    let
      opts = allLibsMeta.__docsOptions or { };
      showIndex = opts.showIndex or true;
      showTitle = opts.showTitle or true;
      cleanMeta = builtins.removeAttrs allLibsMeta [ "__docsOptions" ];
      allSortedLibNames = builtins.sort (a: b: a < b) (builtins.attrNames cleanMeta);
      indexEntries = lib.concatMapStringsSep "\n" (
        name: libToIndexEntry name cleanMeta.${name}
      ) allSortedLibNames;
      libDocs = lib.concatMapStrings (name: libToMarkdown name cleanMeta.${name}) allSortedLibNames;
      libCount = builtins.length allSortedLibNames;

      titleBlock =
        if showTitle then
          ''
            # nix-lib API Reference

            Generated documentation for all defined library functions.

            **Total libs:** ${toString libCount}

          ''
        else
          "";

      indexBlock =
        if showIndex then
          ''
            ## Index

            ${indexEntries}

          ''
        else
          "";
    in
    ''
      ${titleBlock}${indexBlock}
      ${libDocs}
    '';
in
{
  inherit
    nameToAnchor
    libToIndexEntry
    libToMarkdown
    generateMarkdown
    typeToString
    valueToNix
    testsToMarkdown
    ;
}
