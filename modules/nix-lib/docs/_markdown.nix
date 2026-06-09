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

  # Generate index entry for a single lib
  libToIndexEntry =
    name: meta:
    let
      anchor = nameToAnchor name;
      fileLink = if meta.file or null != null then " ([source](${meta.file}))" else "";
    in
    "- [`${name}`](#${anchor})${fileLink}";

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
      testCount = builtins.length (builtins.attrNames (meta.tests or { }));
      testsStr = if testCount > 0 then " (${toString testCount} tests)" else "";
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
    in
    ''
      ### `${name}` {#${anchor}}${visibleStr}
      ${typeStr}
      ${descStr}${testsStr}
      ${fileStr}${exampleStr}
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
    ;
}
