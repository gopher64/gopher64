# Adding translations to Gopher64
1. Copy and modify the `gopher64.pot` file in this directory with your translations.
2. Rename it to `gopher64.po` and put it here:
`data/translations/<lang>/LC_MESSAGES/gopher64.po`

See [here](https://docs.slint.dev/latest/docs/slint/guide/development/translations/#translating-strings) for more details on creating a .po file. I recommend using a program like [poedit](https://poedit.com) to create the .po file from the template.

---

For example, in the .pot file you will see:
```
#: src/ui/gui/about_page.slint:17
msgctxt "NewVersionButton"
msgid "New version available! Click here to download"
msgstr ""
```

A translation will look something like this:
```
#: src/ui/gui/about_page.slint:17
msgctxt "NewVersionButton"
msgid "New version available! Click here to download"
msgstr "Nova versão disponível! Clique aqui para baixar"
```

The only line you modify is the `msgstr`. The `msgctxt` and `msgid` need to remain unchanged.
