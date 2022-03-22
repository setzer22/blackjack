function Plugin_main()
    table.insert(package.loaders or package.searchers, fennel.searcher)
end