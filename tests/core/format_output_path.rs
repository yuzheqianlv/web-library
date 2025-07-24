//  ██████╗  █████╗ ███████╗███████╗██╗███╗   ██╗ ██████╗
//  ██╔══██╗██╔══██╗██╔════╝██╔════╝██║████╗  ██║██╔════╝
//  ██████╔╝███████║███████╗███████╗██║██╔██╗ ██║██║  ███╗
//  ██╔═══╝ ██╔══██║╚════██║╚════██║██║██║╚██╗██║██║   ██║
//  ██║     ██║  ██║███████║███████║██║██║ ╚████║╚██████╔╝
//  ╚═╝     ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝

#[cfg(test)]
mod passing {
    use monolith::core::format_output_path;

    #[test]
    fn as_is() {
        let final_destination = format_output_path(
            "/home/username/Downloads/website.html",
            Some(""),
            false,
        );

        assert_eq!(final_destination, "/home/username/Downloads/website.html");
    }

    #[test]
    fn substitute_title() {
        let final_destination = format_output_path(
            "/home/username/Downloads/%title%.html",
            Some("Document Title"),
            false,
        );

        assert_eq!(
            final_destination,
            "/home/username/Downloads/Document Title.html"
        );
    }

    #[test]
    fn substitute_title_multi() {
        let final_destination = format_output_path(
            "/home/username/Downloads/%title%/%title%.html",
            Some("Document Title"),
            false,
        );

        assert_eq!(
            final_destination,
            "/home/username/Downloads/Document Title/Document Title.html"
        );
    }

    #[test]
    fn sanitize() {
        let final_destination = format_output_path(
            r#"/home/username/Downloads/<>:"|?/%title%.html"#,
            Some(r#"/\<>:"|?"#),
            false,
        );

        assert_eq!(
            final_destination,
            r#"/home/username/Downloads/<>:"|?/__[] - -.html"#
        );
    }

    #[test]
    fn level_up() {
        let final_destination =
            format_output_path("../%title%.html", Some(".Title"), false);

        assert_eq!(final_destination, r#"../Title.html"#);
    }

    #[test]
    fn file_name_extension() {
        let final_destination =
            format_output_path("%title%.%extension%", Some("Title"), false);

        assert_eq!(final_destination, r#"Title.html"#);
    }

    #[test]
    fn file_name_extension_mhtml() {
        let final_destination =
            format_output_path("%title%.%extension%", Some("Title"), true);

        assert_eq!(final_destination, r#"Title.mhtml"#);
    }

    #[test]
    fn file_name_extension_short() {
        let final_destination =
            format_output_path("%title%.%ext%", Some("Title"), false);

        assert_eq!(final_destination, r#"Title.htm"#);
    }

    #[test]
    fn file_name_extension_short_mhtml() {
        let final_destination =
            format_output_path("%title%.%ext%", Some("Title"), true);

        assert_eq!(final_destination, r#"Title.mht"#);
    }
}
