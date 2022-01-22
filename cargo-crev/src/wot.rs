use std::{io, io::Write as _};

use crate::{opts::WotOpts, term, url_to_status_str};
use ::term::color::{BLUE, GREEN, RED, YELLOW};
use anyhow::Result;
use crev_wot::TraverseLogItem::{Edge, Node};

pub fn print_log(wot_opts: WotOpts) -> Result<()> {
    let mut term = term::Term::new();
    let local = crev_lib::Local::auto_create_or_open()?;
    let db = local.load_db()?;
    let trust_set = local.trust_set_for_id(
        wot_opts.for_id.as_deref(),
        &wot_opts.trust_params.clone().into(),
        &db,
    )?;

    if term.stderr_is_tty && term.stdout_is_tty {
        writeln!(
            io::stderr(),
            "{:^43}          {:>6} {:>4}",
            "TRUST-FROM-ID",
            "TRUST",
            "DIST"
        )?;
        writeln!(io::stderr(), "\\_ status URL",)?;
        writeln!(
            io::stderr(),
            "  {:^43} {:>6} {:>6} +{:<3} notes",
            "TRUST-TO-ID",
            "D.TRST",
            "E.TRST",
            "DIS"
        )?;
        writeln!(io::stderr(), "  \\_ status URL",)?;
    }
    for log_item in trust_set.traverse_log {
        match log_item {
            Node(node) => {
                let (status, url) = url_to_status_str(&db.lookup_url(&node.id));

                term.print(format_args!("{}", &node.id), GREEN)?;

                writeln!(
                    io::stdout(),
                    "          {:>6} {:>4}",
                    node.effective_trust,
                    node.total_distance,
                )?;
                writeln!(io::stdout(), "\\_ {} {}", status, url)?;
            }
            Edge(edge) => {
                let (status, url) = url_to_status_str(&db.lookup_url(&edge.to));

                write!(io::stdout(), "  ")?;

                term.print(format_args!("{}", &edge.to), BLUE)?;

                write!(
                    io::stdout(),
                    " {:>6} {:>6} {:>4} ",
                    edge.direct_trust,
                    edge.effective_trust,
                    edge.relative_distance
                        .map(|d| format!("+{}", d))
                        .unwrap_or_else(|| "inf".into()),
                )?;
                if edge.no_change {
                    term.print(format_args!("no change"), YELLOW)?;
                } else {
                    term.print(format_args!("queued"), GREEN)?;
                }
                if edge.ignored_distrusted {
                    write!(io::stdout(), "; ")?;
                    term.print(format_args!("distrusted"), RED)?;
                }
                if edge.ignored_overriden {
                    write!(io::stdout(), "; ")?;
                    term.print(format_args!("overriden"), YELLOW)?;
                }
                if edge.ignored_too_far {
                    write!(io::stdout(), "; ")?;
                    term.print(format_args!("too far"), YELLOW)?;
                }
                if edge.ignored_trust_too_low {
                    write!(io::stdout(), "; ")?;
                    term.print(format_args!("trust too low"), YELLOW)?;
                }

                writeln!(io::stdout(), "")?;

                writeln!(io::stdout(), "  \\_ {} {}", status, url)?;
            }
        }
    }

    Ok(())
}
