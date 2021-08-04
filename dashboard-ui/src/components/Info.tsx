import { useEffect, useState } from "react";
import { getInfo, BuildInfo } from "../utils/ImageProxy";

const Info = () => {
  const [info, setInfo] = useState<BuildInfo>();
  useEffect(() => {
    getInfo().then((i) => setInfo(i));
  }, []);
  return (
    <div className="px-28 py-6">
      <div>Package Version: {info?.package_version} </div>
      <div>Git Version: {info?.git_version} </div>
    </div>
  );
};

export default Info;
